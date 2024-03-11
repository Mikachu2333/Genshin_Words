# coding=utf-8
from pathlib import Path
import pandas as pd

p = Path.cwd().resolve()
collected_text = p.joinpath("test.txt")
out_text = p.joinpath("test_new.txt")
out_df = p.joinpath("pd_new.txt")


def batch_replace(file_path: Path, file_path_new: Path, str_dict: dict):
    """替换词语"""
    file_data = ""  # 初始化
    with open(file_path, "r", encoding="utf-8") as f:
        for line in f:  # line一行行读取替换文本
            for old_str in str_dict:
                new_str = str_dict[old_str]
                line = line.replace(old_str, new_str)
            file_data += line
    with open(file_path_new, "w", encoding="utf-8") as f:  # 写入替换好的文本
        f.write(file_data)
    print("批量替换完成")


need_replace_text = {
    "<color=#F39000FF>": "",
    "</color>": "",
    "「": "",
    "」": "",
    "，": "\n",
    "。": "\n",
    "、": "\n",
    "；": "\n",
    "《": "",
    "》": "\n",
    "（": "\n",
    "）": "",
    "·": "",
    " ": "",
    "…": "",
    "——": "\n",
    "？": "\n",
    "！": "\n",
    "：": "\n",
    "-": "\n",
    "『": "",
    "』": "\n",
    "<i>": "",
    "</i>": "",
    "（test）": "",
    "..": "",
    "...": "",
    ",": "",
    "{REALNAME[ID(1)|HOSTONLY(true)]}": "",
    "#": "",
    "SYUUMATSUGAIDEN": "SYUUMATSU GAIDEN",
    "QUESTCLEAR": "QUEST CLEAR",
    "QUESTFAILED": "QUEST FAILED",
    "Niceboat!": "Nice boat!",
    "DejaVu!": "Deja Vu!",
    "CREDETENEBRIS": "CREDE TENEBRIS",
    "NihilSubCaligineNovum": "Nihil Sub Caligine Novum",
    "MyPrecious": "My Precious",
    "SmellslikeAnimalSpirit!": "Smells like Animal Spirit!",
    "AKillwithoutWater": "A Kill without Water",
    "Eremitisnecredite": "Eremita ne credite",
    "LoveisDestructive": "Love is Destructive",
    "Thisisfine": "This is fine",
    "DanceLikeYouWanttoWin!": "Dance Like You Want to Win!",
    "test": "",
    "(test)": "",
    "TEST": "",
    "<color=#00E1FFFF>": "",
    "(test": "",
    "<color=#FF5E41CC>": "",
    "test)": "",
    "\n\n\n\n\n": "\n",
    "\n\n\n\n": "\n",
    "\n\n\n": "\n",
    "\n\n": "\n",
}

batch_replace(collected_text, out_text, need_replace_text)

df = pd.read_csv(out_text, encoding="utf8", header=None)
df.drop_duplicates(ignore_index=True, inplace=True)
df.to_csv(out_df, encoding="utf8", index=False, index_label=None)
print("Finish")
