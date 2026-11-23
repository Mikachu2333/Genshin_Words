"""保留文本的类别及出处；用于构建可离线复现的词条保护表。"""

import hashlib
import json
import re
import subprocess
from collections import defaultdict
from pathlib import Path

from text_fields import AVATAR_TEXT_FIELDS, TEXT_FIELDS

HAN = r"\u3400-\u4dbf\u4e00-\u9fff\u3005\u3007\uf900-\ufaff\U00020000-\U0002ffff"
HAN_WORD = re.compile(f"^[{HAN}]+$")
HAN_RUN = re.compile(f"[{HAN}]+")
QUOTED = re.compile(r"[「『《](.*?)[」』》]")
TOKENS = re.compile(r"[^.\[\]]+|\[\]")

# 家具名、套装名与其图纸不属于整词保护范围。
NAME_FIELDS = {
    "Achievement.json": ("[].Title",),
    "AchievementGoal.json": ("[].Name",),
    "Chapter.json": ("[].Title", "[].ImageTitle"),
    "DisplayItem.json": ("[].Name",),
    "GachaEvent.json": ("[].Name",),
    "HardChallengeSchedule.json": ("[].Name",),
    "HyperLinkName.json": ("[].Name",),
    "Material.json": ("[].Name",),
    "Monster.json": ("[].Name", "[].Title", "[].Affixes[]"),
    "NameCard.json": ("[].Name",),
    "ProfilePicture.json": ("[].Name",),
    "Reliquary.json": ("[].Name",),
    "ReliquarySet.json": ("[].Name",),
    "Weapon.json": ("[].Name", "[].Affix.Name"),
    "TowerSchedule.json": ("[].BuffName",),
}
AVATAR_NAMES = tuple(
    field for field in AVATAR_TEXT_FIELDS
    if field == "Name" or field.endswith(".Name")
) + (
    "FetterInfo.Title", "FetterInfo.Native",
    "FetterInfo.ConstellationBefore", "FetterInfo.ConstellationAfter",
)


def field_values(node, expression):
    """返回值及具体数组下标，方便从报告定位原始记录。"""
    def walk(value, tokens, location):
        if not tokens:
            if isinstance(value, str) and value.strip():
                yield value, location
            return
        head, *tail = tokens
        if head == "[]" and isinstance(value, list):
            for index, item in enumerate(value):
                yield from walk(item, tail, f"{location}[{index}]")
        elif head == "*" and isinstance(value, dict):
            for key, item in value.items():
                yield from walk(item, tail, f"{location}.{key}")
        elif isinstance(value, dict) and head in value:
            yield from walk(value[head], tail, f"{location}.{head}")
    yield from walk(node, TOKENS.findall(expression), "$")


def plain_text(text):
    text = text.replace("\\n", "\n")
    text = re.sub(r"<[^>]*>|\{/?LINK[^}]*\}", "", text)
    return re.sub(r"\{[^}]*\}", " ", text)


def name_entries(text):
    """只移除排版引号，保留「慈爱」的教导这样的完整名称。"""
    text = plain_text(text).translate(str.maketrans("", "", "「」『』《》\"“”"))
    return [word for word in HAN_RUN.findall(text) if word]


def field_kind(relative, expression):
    names = AVATAR_NAMES if relative.startswith("Avatar/") else NAME_FIELDS.get(relative, ())
    if expression in names:
        return "name"
    if (relative == "Achievement.json" and expression == "[].Description"
            or relative == "Weapon.json" and expression.startswith("[].Affix.")
            or relative.startswith("Avatar/") and expression.startswith("SkillDepot.")):
        return "protected_description"
    if relative.startswith("Furniture") and expression in {"[].Name", "[].Name2"}:
        return "furniture"
    return "prose"


def metadata_records(metadata):
    root = metadata / "Genshin" / "CHS"
    if not root.is_dir():
        raise ValueError(f"找不到元数据目录：{root}")
    for pattern, fields in TEXT_FIELDS.items():
        paths = sorted(root.glob(pattern))
        if not paths:
            raise ValueError(f"保护表所需数据缺失：{root / pattern}")
        for path in paths:
            relative = path.relative_to(root).as_posix()
            data = json.loads(path.read_text(encoding="utf-8"))
            for expression in fields:
                for text, location in field_values(data, expression):
                    kind = field_kind(relative, expression)
                    # Material 也含摆设图纸，不能通过另一张表把家具重新保护起来。
                    if relative == "Material.json" and kind == "name":
                        index = int(re.match(r"\$\[(\d+)\]", location)[1])
                        if data[index].get("TypeDescription") == "摆设图纸":
                            kind = "furniture"
                    yield text, kind, f"Snap.Metadata/{relative}:{location}"


def make_trie(words):
    root = {}
    for word in words:
        node = root
        for char in word:
            node = node.setdefault(char, {})
        node[None] = word
    return root


def matches(text, trie):
    for start in range(len(text)):
        node = trie
        for char in text[start:]:
            if char not in node:
                break
            node = node[char]
            if None in node:
                yield node[None]


def build_sources(metadata, npc_sources, words, revision=None):
    if not npc_sources.is_file():
        raise ValueError(f"缺少 NPC 来源表：{npc_sources}；请先运行 Genshin_NPC_DATA/build.py")
    sources = defaultdict(set)
    furniture = defaultdict(set)
    trie = make_trie(words)
    corpus = []
    records = list(metadata_records(metadata))
    furniture_names = {
        word for text, kind, _ in records if kind == "furniture"
        for word in name_entries(text)
    }

    def add(word, source):
        if len(word) >= 2 and HAN_WORD.fullmatch(word):
            sources[word].add(source)

    def consume(text, kind, source):
        clean = plain_text(text)
        corpus.append(clean)
        if kind == "name":
            for word in name_entries(text):
                add(word, source)
            # 旧提取器会切开引号；也保护现有名称的完整片段。
            for word in HAN_RUN.findall(clean):
                if word in words:
                    add(word, source)
        elif kind == "furniture":
            for word in name_entries(text):
                furniture[word].add(source)
            for word in HAN_RUN.findall(clean):
                if word in words:
                    furniture[word].add(source)
        elif kind == "protected_description":
            # 不用删除数值/战斗语句的正则，避免技能特效、成就详情被误删。
            for word in matches(clean, trie):
                add(word, source)
            for word in HAN_RUN.findall(clean):
                if len(word) <= 10:
                    add(word, source)
        # 对白、介绍中引号明确标出的组织、人名、地点等也作为保护证据。
        # 家具名称本身允许拆分，家具介绍中的专名仍可受保护。
        if kind not in {"name", "furniture"}:
            for quoted in QUOTED.findall(clean):
                for word in name_entries(quoted):
                    # 图纸介绍会重复引用家具名；不能因此把可拆家具再次锁为专名。
                    # 成就/技能详情及真正的名称字段仍走上面的强保护分支。
                    if word not in furniture_names:
                        add(word, source + " (引号词)")

    for text, kind, source in records:
        consume(text, kind, source)
    revisions = set()
    for line in npc_sources.read_text(encoding="utf-8").splitlines():
        record = json.loads(line)
        revisions.add(record["revision"])
        consume(record["text"], record["kind"],
                f"Genshin_NPC_DATA/{record['table']}:{record['id']}.{record['field']}")
    if revision is None:
        revision = subprocess.run(
            ["git", "rev-parse", "HEAD"], cwd=metadata,
            capture_output=True, check=True, text=True,
        ).stdout.strip()
    return {
        "metadata_revision": revision,
        "npc_revisions": sorted(revisions),
        "npc_sha256": hashlib.sha256(npc_sources.read_bytes()).hexdigest(),
        "terms": {word: sorted(refs) for word, refs in sorted(sources.items())},
        "furniture": {word: sorted(refs) for word, refs in sorted(furniture.items())},
    }, "\n".join(corpus)


def write_sources(path, sources):
    """每个词一行，最多存三条定位证据，避免高频词重复引用撑大版本差异。"""
    path.parent.mkdir(parents=True, exist_ok=True)
    metadata = {key: value for key, value in sources.items() if key not in {"terms", "furniture"}}
    chunks = ["{\n" + ",\n".join(
        "  " + json.dumps(key) + ": " + json.dumps(value, ensure_ascii=False)
        for key, value in metadata.items()
    )]
    for category in ("terms", "furniture"):
        separator = ",\n" if metadata or category == "furniture" else ""
        chunks.append(f'{separator}  "{category}": {{\n')
        chunks.append(",\n".join(
            "    " + json.dumps(word, ensure_ascii=False) + ": " + json.dumps(refs[:3], ensure_ascii=False)
            for word, refs in sorted(sources.get(category, {}).items())
        ))
        chunks.append("\n  }")
    chunks.append("\n}\n")
    temporary = path.with_name(path.name + ".tmp")
    temporary.write_text("".join(chunks), encoding="utf-8", newline="\n")
    temporary.replace(path)
