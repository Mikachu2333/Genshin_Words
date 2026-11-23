"""按来源保护专名、删除可组合的常见词，并按词汇拆分超过十字的条目。

默认只预览，--write 才改词典。运行只需 Python 标准库。
"""

import argparse
import hashlib
import json
import math
from collections import defaultdict
from dataclasses import dataclass
from pathlib import Path

from curation_sources import HAN_WORD, build_sources, write_sources

BASE_DIR = Path(__file__).resolve().parent
GLUE = frozenset("的之与和及在是又而着了吧们我你他她它这那也都就很不对把将向从让被或于以为所个一")


@dataclass(frozen=True)
class Entry:
    word: str
    columns: tuple[str, ...] = ()

    def __post_init__(self):
        if not HAN_WORD.fullmatch(self.word):
            raise ValueError(f"不是纯汉字词条：{self.word!r}")

    def render(self):
        return "\t".join((self.word, *self.columns))


def read_dictionary(path):
    lines = path.read_text(encoding="utf-8").splitlines()
    try:
        boundary = lines.index("...") + 1
    except ValueError as error:
        raise ValueError(f"{path} 缺少 Rime 头部结束标记 ...") from error
    header = lines[:boundary]
    entries = []
    for number, line in enumerate(lines[boundary:], boundary + 1):
        if not line.strip() or line.startswith("#"):
            continue
        word, *columns = line.split("\t")
        if not HAN_WORD.fullmatch(word):
            raise ValueError(f"{path}:{number} 不是纯汉字词条：{word!r}")
        entries.append(Entry(word, tuple(columns)))
    return header, entries


def write_dictionary(path, header, entries):
    content = "\n".join(header) + "\n\n"
    content += "".join(entry.render() + "\n" for entry in entries)
    temporary = path.with_name(path.name + ".tmp")
    temporary.write_text(content, encoding="utf-8", newline="\n")
    temporary.replace(path)


class Curator:
    def __init__(self, rules, sources, lexicon):
        self.rules = rules
        self.limit = rules["max_length"]
        if self.limit != 10:
            raise ValueError("词库规定最长十字，max_length 必须为 10")
        self.sources = sources["terms"]
        self.furniture = sources.get("furniture", {})
        self.lexicon = lexicon
        self.keep = set(rules["keep"])
        self.common = set(rules["common_words"])
        self.splits = rules["split"]
        self.corrections = rules["corrections"]
        self.exclude = rules.get("exclude", {})
        self.keep.update(record["replacement"] for record in self.corrections.values())
        self.composable = set(rules.get("composable", []))
        self.lexicon_total = sum(frequency for frequency, _ in self.lexicon.values()) or 1
        for word, parts in self.splits.items():
            if (len(parts) < 2 or "".join(parts) != word
                    or any(not HAN_WORD.fullmatch(p) or len(p) > self.limit for p in parts)):
                raise ValueError(f"拆分规则必须逐字覆盖原词，且每段不超过十字：{word}")
        for wrong, record in self.corrections.items():
            correct = record["replacement"]
            if (wrong == correct or not HAN_WORD.fullmatch(correct)
                    or not record.get("evidence") or correct in self.corrections):
                raise ValueError(f"纠错必须有证据且不能形成替换链：{wrong}")
        for word, record in self.exclude.items():
            if not record.get("reason") or not record.get("evidence"):
                raise ValueError(f"排除词条必须注明原因与证据：{word}")
        if self.exclude.keys() & (self.keep | self.splits.keys() | self.corrections.keys()):
            raise ValueError("排除词条不能同时要求保留、拆分或纠错")
        if self.keep & self.splits.keys():
            raise ValueError("同一词不能同时要求保留和拆分")
        # 长词拆出的实词在下次运行仍须保留，才能重复运行而不继续侵蚀结果。
        self.long_parts = {
            part for word, parts in self.splits.items() if len(word) > self.limit
            for part in parts if len(part) > 1 and not self.easy(part)
        }
        self.families = self.furniture_families()
        self.family_parts = {
            part for parts in self.families.values() for part in parts
            if len(part) > 1 and not self.common_parts(part)
        }

    @classmethod
    def load(cls, rules_path=BASE_DIR / "curation_rules.json",
             sources_path=BASE_DIR / "data/protected_terms.json",
             lexicon_path=BASE_DIR / "data/jieba.dict.txt"):
        lexicon = {}
        for line in lexicon_path.read_text(encoding="utf-8").splitlines():
            word, frequency, tag = line.split()
            if HAN_WORD.fullmatch(word):
                lexicon[word] = (int(frequency), tag)
        return cls(json.loads(rules_path.read_text(encoding="utf-8")),
                   json.loads(sources_path.read_text(encoding="utf-8")), lexicon)

    def easy(self, word):
        if word in self.common:
            return True
        if len(word) == 1:
            return word in GLUE
        frequency, tag = self.lexicon.get(word, (0, ""))
        # 通用词表收录不等于常见词；人名、地名、团体名标记不作删除依据。
        return frequency >= self.rules["common_min_frequency"] and tag not in {"nr", "nrfg", "nrt", "ns", "nt", "nz"}

    def protected(self, word):
        return (word in self.keep or word in self.sources or word in self.long_parts
                or word in self.family_parts)

    def common_parts(self, word):
        """整段必须由高频词和连接字覆盖，才可交给基础词库输入。"""
        best = {len(word): []}
        for start in range(len(word) - 1, -1, -1):
            for end in range(len(word), start, -1):
                part = word[start:end]
                if end in best and self.easy(part):
                    best[start] = [part, *best[end]]
                    break
        return best.get(0)

    def vocabulary_covered(self, word):
        """无标点边界需要多字词覆盖余段，避免“圆圆 / 帽大厨”这样的断词。"""
        reachable = {len(word)}
        for start in range(len(word) - 1, -1, -1):
            for end in range(start + 1, len(word) + 1):
                part = word[start:end]
                if end in reachable and ((len(part) > 1 and (part in self.lexicon or part in self.sources))
                                         or part in GLUE):
                    reachable.add(start)
                    break
        return 0 in reachable

    def furniture_families(self):
        """从完整家具名称学习共有前后缀，保留差异部分，不维护逐词删除名单。"""
        minimum = self.rules.get("furniture_min_family", 2)
        groups = defaultdict(set)
        for word in self.furniture:
            if word in self.sources or word in self.keep or len(word) < 4:
                continue
            for size in range(2, len(word) - 1):
                for side, affix in (("prefix", word[:size]), ("suffix", word[-size:])):
                    core = affix.rstrip("的之") if side == "prefix" else affix.lstrip("的之")
                    remainder = word[size:] if side == "prefix" else word[:-size]
                    if len(core) < 2:
                        continue
                    if len(remainder.strip("的之")) < 2:
                        continue
                    # 共有片段必须有独立词汇依据，不能仅凭两条名称同字开头拆词。
                    explicit_boundary = affix.endswith("的") if side == "prefix" else affix.startswith("的")
                    if not explicit_boundary and not self.vocabulary_covered(remainder):
                        continue
                    if explicit_boundary or core in self.lexicon or core in self.sources or core in self.keep or self.common_parts(core):
                        groups[(side, affix)].add(word)
        candidates = defaultdict(list)
        for (side, affix), words in groups.items():
            boundary = affix.endswith(("的", "之")) if side == "prefix" else affix.startswith(("的", "之"))
            if len(words) < (minimum if boundary else max(3, minimum)):
                continue
            for word in words:
                if side == "prefix":
                    parts = [affix[:-1], affix[-1], word[len(affix):]] if boundary else [affix, word[len(affix):]]
                else:
                    parts = [word[:-len(affix)], affix[0], affix[1:]] if boundary else [word[:-len(affix)], affix]
                if len(parts[0]) > 2 and parts[0].endswith(("的", "之")):
                    parts = [parts[0][:-1], parts[0][-1], *parts[1:]]
                score = 2 if (affix.endswith("的") if side == "prefix" else affix.startswith("的")) else int(boundary)
                candidates[word].append(((score, len(affix), len(words), side == "prefix"), parts))
        selected = {word: max(choices, key=lambda item: item[0])[1] for word, choices in candidates.items()}
        selected.update({word: parts for word, parts in self.splits.items() if word in self.furniture})
        flattened = {}

        def expand(word):
            if word not in selected and word not in self.splits:
                return [word]
            if word not in flattened:
                children = self.splits[word] if word in self.splits else selected[word]
                flattened[word] = [part for child in children for part in expand(child)]
            return flattened[word]

        for word in selected:
            expand(word)
        return flattened

    def easy_split(self, word, retained):
        """只用已保留的更短词和高频常用词解释整条；不允许未知字逐字凑齐。"""
        best = {len(word): (0, [])}
        for start in range(len(word) - 1, -1, -1):
            choices = []
            for end in range(start + 1, len(word) + 1):
                part = word[start:end]
                if part == word or end not in best:
                    continue
                if part in retained or self.easy(part):
                    cost, tail = best[end]
                    choices.append((cost + 1, [part, *tail]))
            if choices:
                best[start] = min(choices, key=lambda item: (item[0], -len(item[1][0]), item[1]))
        parts = best.get(0, (0, []))[1]
        # 单独“之”“的”等连接字不构成删除理由：阿赫玛尔之须不能按单字拼出来。
        if parts and any(len(part) >= 2 and self.easy(part) for part in parts):
            return parts
        return None

    def lexical_split(self, word):
        """动态规划分词：优先完整专名，再按词频选词；无法分词则要求补规则。"""
        if word in self.splits:
            return self.splits[word]
        best = {len(word): ((0, 0.0), [])}
        total = self.lexicon_total
        for start in range(len(word) - 1, -1, -1):
            choices = []
            for end in range(start + 1, min(len(word), start + self.limit) + 1):
                part = word[start:end]
                if end not in best:
                    continue
                protected = len(part) > 1 and self.protected(part)
                if not protected and part not in self.lexicon:
                    continue
                cost, tail = best[end]
                frequency = self.lexicon.get(part, (1, ""))[0]
                score = (cost[0] - (len(part) if protected else 0),
                         cost[1] + math.log(total / max(1, frequency)))
                choices.append((score, [part, *tail]))
            if choices:
                best[start] = min(choices, key=lambda item: item[0])
        if 0 not in best:
            raise ValueError(f"无法按词汇完整拆分 {word!r}，请在 curation_rules.json 的 split 中补充边界")
        return best[0][1]

    def curate(self, entries):
        by_word = {}
        actions = []
        correction_review = []
        for entry in entries:
            if entry.word in self.exclude:
                record = self.exclude[entry.word]
                actions.append({"word": entry.word, "action": "remove", "parts": [],
                                "reason": record["reason"], "evidence": record["evidence"]})
                continue
            if entry.word in self.corrections:
                correction = self.corrections[entry.word]
                target = correction["replacement"]
                if entry.columns and "columns" not in correction:
                    correction_review.append({"word": entry.word, "reason": "correction_reading",
                                              "replacement": target, "message": "纠错规则需提供 columns 手工拼音列"})
                    continue
                actions.append({"word": entry.word, "action": "correct", "parts": [target],
                                "evidence": correction["evidence"]})
                entry = Entry(target, tuple(correction.get("columns", ())))
            by_word.setdefault(entry.word, []).append(entry)
        if correction_review:
            raise ReviewRequired(correction_review)

        # 先展开拆分，使其生成的专名可作为后续组合的基础词。
        forced = {}
        review = []
        for word in by_word:
            if len(word) > self.limit or word in self.splits:
                if any(entry.columns for entry in by_word[word]):
                    review.append({"word": word, "reason": "manual_reading", "source": "long_or_explicit"})
                    continue
                try:
                    forced[word] = self.lexical_split(word)
                except ValueError as error:
                    review.append({"word": word, "reason": "unknown_boundary", "message": str(error)})
            elif word in self.families:
                if not any(entry.columns for entry in by_word[word]):
                    forced[word] = self.families[word]
        if review:
            raise ReviewRequired(review)
        self.splits.update({word: parts for word, parts in forced.items() if len(word) > self.limit})
        long_parts = {part for word, parts in forced.items() if len(word) > self.limit
                      for part in parts if len(part) > 1 and not self.easy(part)}
        self.long_parts.update(long_parts)
        retained = set(long_parts)
        for word, parts in forced.items():
            if len(word) <= self.limit:
                retained.update(p for p in parts if len(p) > 1 and not self.common_parts(p))
        actions_by_word = {}
        for word in sorted(by_word, key=lambda item: (len(item), item)):
            if word in forced:
                action = {"word": word, "action": "split", "parts": forced[word],
                          "reason": ("over_length" if len(word) > self.limit else
                                     "explicit_split" if word in self.splits else "furniture_family")}
            elif self.protected(word):
                retained.add(word)
                continue
            elif self.easy(word) and len(word) > 1:
                action = {"word": word, "action": "remove", "parts": [word], "reason": "common_word"}
            elif (word in self.furniture or word in self.composable) and (parts := self.easy_split(word, retained)):
                action = {"word": word, "action": "remove", "parts": parts, "reason": "composable"}
            else:
                retained.add(word)
                continue
            if word in self.sources:
                action["sources"] = self.sources[word]
            elif word in self.furniture:
                action["sources"] = self.furniture[word]
            actions_by_word[word] = action
            actions.append(action)

        output = []
        seen = set()
        for word, rows in by_word.items():
            if word in actions_by_word:
                continue
            for entry in rows:
                # 去除完全相同的行；不同手工读音仍分别保留。
                if entry not in seen:
                    seen.add(entry)
                    output.append(entry)
        present = {entry.word for entry in output}
        for word in sorted(retained - present):
            if word not in actions_by_word:
                output.append(Entry(word))

        final_words = {entry.word for entry in output}
        if any(len(word) > self.limit for word in final_words):
            raise AssertionError("仍有超过十字的词条")
        for word in by_word:
            if self.protected(word) and len(word) <= self.limit and word not in self.splits and word not in final_words:
                raise AssertionError(f"专名保护失败：{word}")
        for action in actions:
            if action.get("reason") == "composable":
                if any(p not in final_words and not self.easy(p) for p in action["parts"]):
                    raise AssertionError(f"组合依赖未保留：{action['word']}")
        return output, actions

    def review_batch(self, entries):
        """把一个批次的问题一次列全；待复核条目不参与可应用结果。"""
        pending = list(entries)
        review = []
        while True:
            try:
                output, actions = self.curate(pending)
                return output, actions, review
            except ReviewRequired as error:
                review.extend(error.items)
                blocked = {item["word"] for item in error.items}
                pending = [entry for entry in pending if entry.word not in blocked]


class ReviewRequired(ValueError):
    def __init__(self, items):
        self.items = items
        super().__init__("无法按词汇拆分或需确认手工拼音：" + "、".join(item["word"] for item in items))


def save_report(path, entries, output, actions, sources_path, rules_path):
    report = {
        "input_rows": len(entries), "output_rows": len(output),
        "input_words": len({e.word for e in entries}),
        "output_words": len({e.word for e in output}),
        "source_sha256": hashlib.sha256(sources_path.read_bytes()).hexdigest(),
        "rules_sha256": hashlib.sha256(rules_path.read_bytes()).hexdigest(),
        "lexicon_sha256": hashlib.sha256((BASE_DIR / "data/jieba.dict.txt").read_bytes()).hexdigest(),
        "actions": actions,
    }
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n", encoding="utf-8", newline="\n")
    return report


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--dictionary", type=Path, default=BASE_DIR / "yuanshen.dict.yaml")
    parser.add_argument("--rules", type=Path, default=BASE_DIR / "curation_rules.json")
    parser.add_argument("--sources", type=Path, default=BASE_DIR / "data/protected_terms.json")
    parser.add_argument("--report", type=Path, default=BASE_DIR / ".cache/curation_report.json")
    parser.add_argument("--write", action="store_true")
    parser.add_argument("--refresh-sources", action="store_true")
    parser.add_argument("--metadata", type=Path, default=BASE_DIR.parent / "Snap.Metadata")
    parser.add_argument("--npc-sources", type=Path, default=BASE_DIR.parent / "Genshin_NPC_DATA/term_sources.jsonl")
    options = parser.parse_args(argv)
    header, entries = read_dictionary(options.dictionary)
    if options.refresh_sources:
        sources, corpus = build_sources(options.metadata, options.npc_sources, {e.word for e in entries})
        write_sources(options.sources, sources)
        cache = BASE_DIR / ".cache"
        cache.mkdir(exist_ok=True)
        (cache / "curation_corpus.txt").write_text(corpus, encoding="utf-8")
    curator = Curator.load(options.rules, options.sources)
    output, actions = curator.curate(entries)
    # 完整二次运行，检查规则是否会逐轮侵蚀词库。
    repeated, _ = curator.curate(output)
    if set(repeated) != set(output):
        raise AssertionError("二次运行结果不同，请检查组合依赖和长词分段")
    if options.write:
        options.rules.write_text(json.dumps(curator.rules, ensure_ascii=False, indent=2) + "\n", encoding="utf-8", newline="\n")
        write_dictionary(options.dictionary, header, output)
    report = save_report(options.report, entries, output, actions, options.sources, options.rules)
    print(f"词条行 {len(entries)} → {len(output)}，不同词 {report['input_words']} → {report['output_words']}")
    print(f"删除 {sum(a['action'] == 'remove' for a in actions)}，拆分 {sum(a['action'] == 'split' for a in actions)}，纠错 {sum(a['action'] == 'correct' for a in actions)}")
    print(f"{'已写入' if options.write else '预览'}；明细：{options.report}")


if __name__ == "__main__":
    main()
