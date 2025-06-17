import re
import os
import datetime


def process_text(text):
    """
    根据注释中的规则处理文本
    """
    # 1. 不以加号开头的行，全部删除
    text = re.sub(r"^(?!\+\s).+", "", text, flags=re.MULTILINE)

    # 2. Cv替换
    text = re.sub(
        r"^.*?(CvChinese|CvJapanese|CvEnglish|CvKorean).*?$",
        "",
        text,
        flags=re.MULTILINE,
    )

    # 3. 仅保留文字部分
    text = re.sub(
        r'^\+\s+[\}\],[\w\d"/\.:\s,\{\[\+-]*(.*?)[",]*$',
        r"\1",
        text,
        flags=re.MULTILINE,
    )

    # 4. 空行替换
    text = re.sub(r"\n\n", "\n", text)

    # 5. 颜色标签等转为文字
    text = re.sub(r"<[\w\d=#]+>(.*?)</[\w]+>", r"\1", text)

    # 6. 替换时间说明
    text = re.sub(r"\|[\{\w\d:\}秒]+", "", text)

    # 7. 替换参数
    text = re.sub(r"[\|\+/]\{param\d+:\w*\d*\w{0,1}\}[点攻击力秒]*", "", text)

    # 8. 替换link
    text = re.sub(r"(\{LINK){0,1}#\w*\d*\}(.*?)\{/LINK\}", r"\2", text)

    # 9. 查缺补漏
    text = re.sub(r"\{LINK", "", text)

    # 10. 替换标点
    text = re.sub(r"[~#。：？『』，—；…「」、！（）\\n]", "\n", text)

    # 11. 删除含数字的部分
    text = re.sub(
        r".*?(提高|触发|迸发|夜魂加持|恢复生命值|额外回复|夜魂值|队伍后台|装备者|此效果|该效果|命中|冷却时间).*?\d{0,1}.*",
        "",
        text,
        flags=re.MULTILINE,
    )

    # 12. 删除部分
    text = re.sub(r".*元素伤害.*", "", text, flags=re.MULTILINE)

    # 13. 查缺补漏
    text = re.sub(r".*\d+%{0,1}.*", "", text, flags=re.MULTILINE)

    # 14. 删除道具说明
    text = re.sub(
        r".*(范围伤害|步骤详实|元素精通|段伤害|人游戏|生效|任务道具|进行治疗|的效果|真实伤害).*",
        "",
        text,
        flags=re.MULTILINE,
    )

    # 15. 删除单字
    text = re.sub(r"^.$", "", text, flags=re.MULTILINE)

    # 16. 查漏补缺
    text = re.sub(r"^·", "", text, flags=re.MULTILINE)

    # 17. 查漏补缺
    text = re.sub(r"-$", "", text, flags=re.MULTILINE)

    # 18. 空行替换
    text = re.sub(r"\n\n", "\n", text)
    text = re.sub(r"\n\n", "\n", text)
    text = re.sub(r"\n\n", "\n", text)
    text = re.sub(r"\n\n", "\n", text)

    return text


def main():
    """
    主函数，处理用户输入的文件
    """
    try:
        # 获取用户输入的文件路径
        file_path = "./new.diff"

        # 检查文件是否存在
        if not os.path.exists(file_path):
            print(f"错误: 文件 '{file_path}' 不存在")
            return

        # 读取文件内容
        with open(file_path, "r", encoding="utf-8") as file:
            original_text = file.read()

        print(f"正在处理文件: {file_path}")
        print(f"原始文本长度: {len(original_text)} 字符")

        # 处理文本
        processed_text = process_text(original_text)

        print(f"处理后文本长度: {len(processed_text)} 字符")

        # 生成输出文件名
        output_path = "after.txt"

        # 保存处理后的文本
        with open(output_path, "w", encoding="utf-8") as file:
            file.write(processed_text)

        print(f"处理完成")

    except Exception as e:
        print(f"处理文件时发生错误: {str(e)}")


if __name__ == "__main__":
    main()
