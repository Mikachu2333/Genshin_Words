"""按版本批量提取、分类、分词和去重。默认生成预览；--apply 更新两份词库。"""

import argparse
import json
import shutil
import subprocess
import sys
from collections import Counter
from pathlib import Path

import extract_words
import merge
from curate_dictionary import BASE_DIR, Curator, Entry, read_dictionary, save_report, write_dictionary


def write_json(path, value):
    path.write_text(json.dumps(value, ensure_ascii=False, indent=2) + "\n", encoding="utf-8", newline="\n")


def prepare_batch(dictionary, candidates, output, rules, sources):
    output.mkdir(parents=True, exist_ok=True)
    header, existing = read_dictionary(dictionary)
    incoming = [Entry(*parse_line(line)) for line in candidates.read_text(encoding="utf-8").splitlines() if line.strip()]
    engine = Curator.load(rules, sources)
    prepared, actions, pending = engine.review_batch(existing + incoming)
    known = {entry.word for entry in existing}
    prepared_words = {entry.word for entry in prepared}
    additions = sorted(prepared_words - known)
    report = save_report(output / "changes.json", existing + incoming, prepared, actions, sources, rules)
    write_json(output / "review.json", pending)
    write_json(output / "proposed_rules.json", engine.rules)
    write_dictionary(output / "preview.dict.yaml", header, prepared)
    (output / "additions.txt").write_text("".join(word + "\n" for word in additions), encoding="utf-8")
    keep = [{"word": word, "reason": "protected" if word in engine.sources else "retained",
             "sources": engine.sources.get(word, [])[:3]} for word in additions]
    write_json(output / "retained.json", keep)
    family_groups = {}
    for action in actions:
        if action.get("reason") == "furniture_family":
            parts = action["parts"]
            key = " / ".join(parts[:-1])
            family_groups.setdefault(key, []).append({"word": action["word"], "parts": parts})
    write_json(output / "furniture_families.json", family_groups)
    counts = Counter(action.get("reason", action["action"]) for action in actions)
    summary = [
        "# 版本词库整理预览", "",
        f"输入 {len(incoming)} 行候选，现有 {len(existing)} 行，整理后 {len(prepared)} 行。",
        f"新增 {len(additions)} 个词，待复核 {len(pending)} 项。", "",
        "| 处理 | 数量 |", "| --- | --- |",
        *[f"| {kind} | {count} |" for kind, count in sorted(counts.items())], "",
        "- `changes.json`：逐词变化及来源；`furniture_families.json`：家具共有片段和分段。",
        "- `retained.json`：保留的新词及保护来源；`additions.txt`：准备加入的词。",
        "- `review.json`：需要补词汇边界或读音的问题；`preview.dict.yaml`：排除待复核项后的预览。",
        "- 将修订写入 `curation_rules.json` 的 `keep`、`split`、`common_words` 或 `corrections`，然后重跑。", "",
    ]
    if pending:
        summary += ["待复核项未进入预览；解决后才能应用到正式词库。", ""]
    (output / "summary.md").write_text("\n".join(summary), encoding="utf-8")
    return report, pending


def parse_line(line):
    word, *columns = line.split("\t")
    return word, tuple(columns)


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("revs", nargs="*", help="Snap.Metadata 的旧、新提交；使用 --candidates 时可省略")
    parser.add_argument("--metadata", type=Path, default=BASE_DIR.parent / "Snap.Metadata")
    parser.add_argument("--npc-project", type=Path, default=BASE_DIR.parent / "Genshin_NPC_DATA")
    parser.add_argument("--npc-repo", type=Path, default=BASE_DIR.parent / "animegamedata2")
    parser.add_argument("--npc-before")
    parser.add_argument("--npc-after")
    parser.add_argument("--npc-sources", type=Path)
    parser.add_argument("--candidates", type=Path, help="重放已提取的词表，跳过数据提取")
    parser.add_argument("--sources", type=Path, help="重放时使用的保护表；默认读取候选文件同目录的保护表")
    parser.add_argument("--output", type=Path, default=BASE_DIR / ".cache/update")
    parser.add_argument("--apply", action="store_true")
    options = parser.parse_args(argv)
    if bool(options.npc_before) != bool(options.npc_after):
        parser.error("--npc-before 和 --npc-after 必须一起提供")
    if options.candidates is None and len(options.revs) != 2:
        parser.error("需要提供两个 Snap.Metadata 提交，或使用 --candidates 重放词表")
    destination = options.output.resolve()
    destination.mkdir(parents=True, exist_ok=True)
    npc_sources = options.npc_sources or options.npc_project / "term_sources.jsonl"
    if options.npc_before:
        npc_output = destination / "npc"
        subprocess.run([
            sys.executable, str(options.npc_project / "build.py"), options.npc_before, options.npc_after,
            "--repo", str(options.npc_repo), "--output", str(npc_output),
        ], check=True)
        npc_sources = npc_output / "term_sources.jsonl"
        extract_words.MANUAL_TEXT_SOURCES = (npc_output / "final_npc.txt",)
    dictionary = BASE_DIR / "yuanshen.dict.yaml"
    candidates = options.candidates
    sources_path = options.sources
    if candidates is None:
        candidates = destination / "candidates.txt"
        extract_words.DICTIONARY_PATH = dictionary
        extract_words.OUTPUT_PATH = candidates
        extract_words.RAW_TEXT_PATH = destination / "new_texts.jsonl"
        sources_path = destination / "protected_terms.json"
        extract_words.SOURCE_OUTPUT_PATH = sources_path
        extract_words.SNAPSHOT_ROOT = BASE_DIR / ".snapshots"
        extract_words.main([*options.revs, "--metadata", str(options.metadata), "--npc-sources", str(npc_sources)])
    elif sources_path is None:
        sources_path = candidates.resolve().parent / "protected_terms.json"
        if not sources_path.is_file():
            sources_path = BASE_DIR / "data/protected_terms.json"
    report, pending = prepare_batch(
        dictionary, candidates, destination,
        BASE_DIR / "curation_rules.json", sources_path,
    )
    print(f"整理结果 {report['output_words']} 个词，待复核 {len(pending)} 项；{destination / 'summary.md'}")
    if options.apply:
        if pending:
            parser.exit(2, "待复核清单尚未解决，正式词库未更新。\n")
        merge.merge(dictionary, candidates, sources_path=sources_path)
        if sources_path.resolve() != (BASE_DIR / "data/protected_terms.json").resolve():
            shutil.copyfile(sources_path, BASE_DIR / "data/protected_terms.json")
    else:
        print(f"复用本批候选：python update_dictionary.py --candidates \"{candidates}\" --apply")


if __name__ == "__main__":
    main()
