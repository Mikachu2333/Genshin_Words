"""把审校好的词条追加进 Rime 词典，再跑排序与 Fcitx5 生成。"""

import subprocess
from pathlib import Path

ENTRIES_PATH = Path("after_del.txt")
DICTIONARY_PATH = Path("yuanshen.dict.yaml")
SORT_EXECUTABLE = Path("genshin_pinyin_sort.exe")
PINYIN_EXECUTABLE = Path("genshin_pinyin_add.exe")


def append_entries() -> None:
    with DICTIONARY_PATH.open("a", encoding="utf-8") as dictionary:
        dictionary.write("\n")
        dictionary.write(ENTRIES_PATH.read_text(encoding="utf-8"))
    print(f"已把 {ENTRIES_PATH} 追加到 {DICTIONARY_PATH}")


def main() -> None:
    append_entries()
    # 排序工具会整行去重，同一个词源重复合并也不会越滚越多
    subprocess.run([str(SORT_EXECUTABLE), f"./{DICTIONARY_PATH}"], check=True)
    subprocess.run([str(PINYIN_EXECUTABLE), f"./{DICTIONARY_PATH}"], check=True)


if __name__ == "__main__":
    main()
