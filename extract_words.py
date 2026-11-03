"""从 Snap.Metadata 的两个 commit 快照里取字段文本，做差集后走词条管线。

流程：
1. 按 commit id 把两次快照的 `Genshin/CHS` 解包到 `.snapshots/<hash>/`（可复用）
2. 用 `text_fields.py` 的字段表逐文件取值
3. 新快照的文本集合减去旧快照的文本集合，只留下真正新增的文本
4. 与手工维护的 `final_npc.txt` 一起走 `replace_self.py` 的规则并去重
5. 输出 `after_del.txt`，等待人工审校
"""

import argparse
import fnmatch
import io
import json
import os
import re
import shutil
import subprocess
import tarfile
from collections.abc import Iterator
from pathlib import Path

from replace_self import (
    collapse_blank_lines,
    delete_chars,
    delete_fight,
    delete_items,
    delete_none_chinese,
    delete_num,
    delete_style,
    keep_han_entries,
    replace_percent,
    replace_text_line,
    replace_to_newline,
)
from text_fields import TEXT_FIELDS

DEFAULT_METADATA_PATH = Path("F:/LanguageLearning/Snap.Metadata")
# main() 会按 `SNAP_METADATA_PATH` 或 `--metadata` 覆盖它
SNAP_METADATA_PATH = DEFAULT_METADATA_PATH
COMMIT_RANGE_PATH = Path("ReadMe.md")
# 解包出来的快照与原始新增文本都放在工作区里，方便对照 JSON 排查
SNAPSHOT_ROOT = Path(".snapshots")
# 每条原始文本一行 JSON，字段值里的换行不会破坏这个文件，去掉正则后可以直接重放
RAW_TEXT_PATH = Path(".cache/new_texts.jsonl")
OUTPUT_PATH = Path("after_del.txt")
# 手工维护的词源，每个版本自行更新，与 JSON 新增文本走同一套切分规则
MANUAL_TEXT_SOURCES: tuple[Path, ...] = (Path("final_npc.txt"),)

TEXT_SUBDIR = Path("Genshin") / "CHS"
COMMIT_RANGE_PATTERN = re.compile(r"git diff ([0-9a-f]{7,40})\.\.([0-9a-f]{7,40})")
# 字段表达式里只区分「按名字下钻」和「展开一层」，`[]` 展开数组，`*` 展开对象的全部值
FIELD_PATH_TOKEN = re.compile(r"[^.\[\]]+|\[\]")
REPORT_FILE_LIMIT = 20


def git(*arguments: str) -> bytes:
    return subprocess.run(
        ["git", "-C", str(SNAP_METADATA_PATH), *arguments],
        capture_output=True,
        check=True,
    ).stdout


def resolve_rev(rev: str) -> str:
    try:
        return git("rev-parse", "--verify", f"{rev}^{{commit}}").decode().strip()
    except subprocess.CalledProcessError as error:
        raise SystemExit(f"Snap.Metadata 里找不到 commit：{rev}") from error


def commit_subject(rev: str) -> str:
    return git("log", "-1", "--format=%s", rev).decode("utf-8").strip()


def commit_range() -> tuple[str, str]:
    readme = COMMIT_RANGE_PATH.read_text(encoding="utf-8")
    match = COMMIT_RANGE_PATTERN.search(readme)
    if match is None:
        raise ValueError(
            f"{COMMIT_RANGE_PATH} 里找不到 `git diff <old>..<new>` 形式的 commit 区间，"
            "也可以直接在命令行给出两个 commit id"
        )
    return match.group(1), match.group(2)


def extract_snapshot(rev: str, rebuild: bool = False) -> tuple[str, Path]:
    """把 commit 下的 CHS 快照解包到 `.snapshots/<完整 hash>/`，返回 hash 与目录。"""
    resolved = resolve_rev(rev)
    destination = SNAPSHOT_ROOT / resolved
    content = destination / TEXT_SUBDIR
    if rebuild and destination.exists():
        shutil.rmtree(destination)
    if not content.is_dir():
        destination.mkdir(parents=True, exist_ok=True)
        archive = git(
            "archive", "--format=tar", resolved, "--", TEXT_SUBDIR.as_posix()
        )
        with tarfile.open(fileobj=io.BytesIO(archive)) as tar:
            tar.extractall(destination, filter="data")
    if not content.is_dir():
        raise FileNotFoundError(f"{resolved} 里没有 {TEXT_SUBDIR}，快照解包失败")
    return resolved, destination


def field_spec_for(relative_path: str) -> tuple[str, ...] | None:
    """先按文件名精确匹配，再按 glob 匹配；没列出的文件不取词。"""
    if relative_path in TEXT_FIELDS:
        return TEXT_FIELDS[relative_path]
    for pattern, fields in TEXT_FIELDS.items():
        if any(marker in pattern for marker in "*?[") and fnmatch.fnmatch(
            relative_path, pattern
        ):
            return fields
    return None


def descends(node: object, tokens: list[str]) -> Iterator[object]:
    if not tokens:
        yield node
        return
    head, rest = tokens[0], tokens[1:]
    if head == "[]":
        if isinstance(node, list):
            for item in node:
                yield from descends(item, rest)
    elif head == "*":
        if isinstance(node, dict):
            for value in node.values():
                yield from descends(value, rest)
    elif isinstance(node, dict) and head in node:
        yield from descends(node[head], rest)


def field_texts(node: object, expression: str) -> Iterator[str]:
    for value in descends(node, FIELD_PATH_TOKEN.findall(expression)):
        if isinstance(value, str) and value.strip():
            yield value


def read_texts(path: Path, fields: tuple[str, ...]) -> list[str]:
    data = json.loads(path.read_text(encoding="utf-8"))
    values: list[str] = []
    for expression in fields:
        values.extend(field_texts(data, expression))
    return values


def snapshot_entries(snapshot: Path) -> Iterator[tuple[str, list[str]]]:
    """按文件路径顺序产出（相对路径，该文件取到的全部文本）。"""
    root = snapshot / TEXT_SUBDIR
    for path in sorted(root.rglob("*.json")):
        relative = path.relative_to(root).as_posix()
        fields = field_spec_for(relative)
        if fields is None:
            continue
        yield relative, read_texts(path, fields)


def new_text_groups(
    old_snapshot: Path, new_snapshot: Path
) -> list[tuple[str, list[str]]]:
    """新快照里出现、旧快照里没有的文本，按文件分组并保持 JSON 中的顺序。"""
    known: set[str] = set()
    for _, values in snapshot_entries(old_snapshot):
        known.update(values)
    groups: list[tuple[str, list[str]]] = []
    reported: set[str] = set()
    for relative, values in snapshot_entries(new_snapshot):
        fresh = [
            value
            for value in values
            if value not in known and value not in reported
        ]
        reported.update(fresh)
        if fresh:
            groups.append((relative, fresh))
    return groups


def manual_text_values() -> list[str]:
    values: list[str] = []
    for source in MANUAL_TEXT_SOURCES:
        values.extend(
            line.strip()
            for line in source.read_text(encoding="utf-8").splitlines()
            if line.strip()
        )
    return values


def process_text(text: str) -> str:
    """对新增文本套用词条规则，顺序与过去的 diff 流程一致。"""
    text = delete_none_chinese(text)
    # 表格行要在 delete_style 删掉 |{param…} 之前识别，否则只剩表头文字，无法与正文区分
    text = delete_fight(text)
    text = delete_style(text)
    text = delete_chars(text)
    text = delete_items(text)
    text = replace_text_line(text)
    text = delete_num(text)
    text = replace_to_newline(text)
    text = replace_percent(text)
    text = delete_none_chinese(text)
    text = replace_to_newline(text)
    return keep_han_entries(collapse_blank_lines(text))


def report_groups(groups: list[tuple[str, list[str]]]) -> None:
    total = sum(len(values) for _, values in groups)
    print(f"新增文本 {total} 条，来自 {len(groups)} 个文件")
    for relative, values in groups[:REPORT_FILE_LIMIT]:
        print(f"  {relative}: {len(values)} 条")
    if len(groups) > REPORT_FILE_LIMIT:
        print(f"  …… 另有 {len(groups) - REPORT_FILE_LIMIT} 个文件")


def main(argv: list[str] | None = None) -> None:
    parser = argparse.ArgumentParser(
        description="按两个 commit 的 Snap.Metadata 快照取新增文本"
    )
    parser.add_argument(
        "revs",
        nargs="*",
        metavar="REV",
        help="旧、新两个 commit id；省略时读 ReadMe.md 里的 `git diff <old>..<new>`",
    )
    parser.add_argument(
        "--rebuild", action="store_true", help="删掉缓存，重新解包两次快照"
    )
    parser.add_argument(
        "--metadata",
        type=Path,
        default=Path(os.environ.get("SNAP_METADATA_PATH", DEFAULT_METADATA_PATH)),
        help="Snap.Metadata 克隆的位置，默认取 SNAP_METADATA_PATH 或代码里的路径",
    )
    arguments = parser.parse_args(argv)
    global SNAP_METADATA_PATH
    SNAP_METADATA_PATH = arguments.metadata
    if not (SNAP_METADATA_PATH / ".git").exists():
        parser.error(f"{SNAP_METADATA_PATH} 不是 Snap.Metadata 的 git 克隆")
    if len(arguments.revs) == 2:
        old_rev, new_rev = arguments.revs
    elif not arguments.revs:
        old_rev, new_rev = commit_range()
    else:
        parser.error("要么给两个 commit id，要么一个都不给（改为读 ReadMe.md）")

    old_hash, old_snapshot = extract_snapshot(old_rev, arguments.rebuild)
    new_hash, new_snapshot = extract_snapshot(new_rev, arguments.rebuild)
    print(f"旧快照 {old_rev} -> {old_hash[:12]}（{commit_subject(old_hash)}）")
    print(f"新快照 {new_rev} -> {new_hash[:12]}（{commit_subject(new_hash)}）")
    print(f"快照目录: {SNAPSHOT_ROOT}")

    groups = new_text_groups(old_snapshot, new_snapshot)
    report_groups(groups)
    values = [value for _, texts in groups for value in texts]

    RAW_TEXT_PATH.parent.mkdir(parents=True, exist_ok=True)
    RAW_TEXT_PATH.write_text(
        "".join(json.dumps(value, ensure_ascii=False) + "\n" for value in values),
        encoding="utf-8",
    )
    print(f"原始新增文本已写入 {RAW_TEXT_PATH}，调整正则后可直接对照")

    manual_values = manual_text_values()
    sources = ", ".join(str(source) for source in MANUAL_TEXT_SOURCES)
    print(f"手工词源 {len(manual_values)} 条（{sources}）")

    entries = process_text("\n".join(values + manual_values)).splitlines()
    words = list(dict.fromkeys(entries))
    OUTPUT_PATH.write_text("\n".join(words) + "\n", encoding="utf-8")
    print(f"去重后词条: {len(words)} 条，已写入 {OUTPUT_PATH}")
    print("请人工审校 after_del.txt 后再运行 merge.py")


if __name__ == "__main__":
    main()
