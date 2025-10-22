import subprocess


def merge(input_file: str):
    # 读取 after.txt 文件内容
    with open(input_file, "r", encoding="utf-8") as f:
        after_content = f.read()

    # 合并内容并写入 yuanshen.dict.yaml
    with open("./yuanshen.dict.yaml", "a", encoding="utf-8") as f:
        f.write("\n")
        f.write(after_content)

    print("文件合并完成！")

    # 运行 genshin_py_sort.exe
    subprocess.run(["./genshin_pinyin_sort.exe", './yuanshen.dict.yaml'])


merge("./after_del.txt")
