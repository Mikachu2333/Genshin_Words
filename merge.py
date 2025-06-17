import datetime
import subprocess

version = input("请输入版本号 (例如: 5.7V1): ").upper().split("V")
header = f"""---
name: yuanshen
version: "{datetime.date.today().strftime("%Y.%m.%d")}v{version[1]} Ver{version[0]}"
sort: origin
use_preset_vocabulary: false
...
"""

try:
    # 读取 yuanshen.dict.yaml 文件从第7行开始的内容
    with open("yuanshen.dict.yaml", "r", encoding="utf-8") as f:
        lines = f.readlines()
        # 从第7行开始读取（索引6）
        dict_content = "".join(lines[6:]) if len(lines) > 6 else ""

    # 读取 after.txt 文件内容（假设是文本文件）
    with open("after.txt", "r", encoding="utf-8") as f:
        after_content = f.read()

    # 合并内容并写入 merge.txt
    with open("merge.txt", "w", encoding="utf-8") as f:
        f.write(dict_content)
        f.write("\n")
        f.write(after_content)

    print("文件合并完成！已生成 merge.txt")

    # 运行 chs_sentences_py_sort.exe
    subprocess.run(["./chs_sentences_py_sort.exe", '"merge.txt"'])

    context = open("merge_out.txt", "r", encoding="utf-8").read()
    with open("yuanshen.dict.yaml", "w", encoding="utf-8") as f:
        f.write(header)
        f.write("\n")
        f.write(context)


except FileNotFoundError as e:
    print(f"文件未找到: {e}")
except Exception as e:
    print(f"发生错误: {e}")
