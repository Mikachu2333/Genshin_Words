"""合并审校词表并应用来源保护规则；两个格式都生成成功后才替换正式词库。"""

import argparse
import json
import shutil
import subprocess
import tempfile
from pathlib import Path

from curate_dictionary import Curator, Entry, read_dictionary, save_report, write_dictionary
from curation_sources import build_sources, write_sources

BASE_DIR = Path(__file__).resolve().parent


def merge(dictionary, additions=None, refresh_sources=False,
          metadata=BASE_DIR.parent / "Snap.Metadata",
          npc_sources=BASE_DIR.parent / "Genshin_NPC_DATA/term_sources.jsonl",
          sources_path=BASE_DIR / "data/protected_terms.json"):
    header, entries = read_dictionary(dictionary)
    if additions is not None:
        for line in additions.read_text(encoding="utf-8").splitlines():
            if line.strip():
                word, *columns = line.split("\t")
                entries.append(Entry(word, tuple(columns)))
    rules_path = BASE_DIR / "curation_rules.json"
    rules = json.loads(rules_path.read_text(encoding="utf-8"))
    curator = Curator.load(rules_path, sources_path)
    sources = None
    if refresh_sources:
        sources, _ = build_sources(metadata, npc_sources, {entry.word for entry in entries})
        curator = Curator(rules, sources, curator.lexicon)
    output, actions = curator.curate(entries)
    repeated, _ = curator.curate(output)
    if set(output) != set(repeated):
        raise ValueError("整理规则重复运行不稳定，词库未替换")
    with tempfile.TemporaryDirectory(prefix="curation-", dir=dictionary.parent) as directory:
        staging = Path(directory)
        staged_dictionary = staging / "yuanshen.dict.yaml"
        write_dictionary(staged_dictionary, header, output)
        shutil.copyfile(BASE_DIR / "pinyin_rules.json", staging / "pinyin_rules.json")
        for executable in ("genshin_pinyin_sort.exe", "genshin_pinyin_add.exe"):
            subprocess.run([str(BASE_DIR / executable), str(staged_dictionary)], cwd=staging, check=True)
        staged_fcitx = staging / "yuanshen_fcitx5.txt"
        if not staged_fcitx.is_file() or not staged_fcitx.stat().st_size:
            raise ValueError("Fcitx5 生成器没有产出词库")
        # Windows 上 TemporaryDirectory 的私有 ACL 会随移动文件带出。
        # 先复制到目标父目录，使待发布文件继承工作区权限，再原子替换。
        publishing = []
        try:
            for staged, destination in (
                (staged_dictionary, dictionary),
                (staged_fcitx, dictionary.with_name("yuanshen_fcitx5.txt")),
            ):
                temporary = destination.with_name(destination.name + ".tmp")
                shutil.copyfile(staged, temporary)
                publishing.append((temporary, destination))
            for temporary, destination in publishing:
                temporary.replace(destination)
        finally:
            for temporary, _ in publishing:
                temporary.unlink(missing_ok=True)
    if sources is not None:
        write_sources(sources_path, sources)
    rules_path.write_text(json.dumps(curator.rules, ensure_ascii=False, indent=2) + "\n", encoding="utf-8", newline="\n")
    save_report(BASE_DIR / ".cache/merge_report.json", entries, output, actions, sources_path, rules_path)
    print(f"词条行 {len(entries)} → {len(output)}；Rime 与 Fcitx5 词库已更新")


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--dictionary", type=Path, default=BASE_DIR / "yuanshen.dict.yaml")
    parser.add_argument("--entries", type=Path, default=BASE_DIR / "after_del.txt")
    parser.add_argument("--no-append", action="store_true", help="只整理现有词库，不合并 after_del.txt")
    parser.add_argument("--refresh-sources", action="store_true", help="更新版本时读取本地两个数据源重建保护表")
    parser.add_argument("--metadata", type=Path, default=BASE_DIR.parent / "Snap.Metadata")
    parser.add_argument("--npc-sources", type=Path, default=BASE_DIR.parent / "Genshin_NPC_DATA/term_sources.jsonl")
    options = parser.parse_args(argv)
    merge(options.dictionary.resolve(), None if options.no_append else options.entries,
          options.refresh_sources, options.metadata, options.npc_sources)


if __name__ == "__main__":
    main()
