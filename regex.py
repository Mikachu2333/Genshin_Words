import os

from replace_self import *
from delete_repeat import delete_duplicates


def process_text(text):
    """
    根据注释中的规则处理文本
    """

    text = delete_sum_symbol(text)

    text = delete_cv(text)
    text = delete_none_chinese(text)
    text = delete_style(text)
    text = delete_fight(text)

    text = extract_content(text)

    text = delete_chars(text)
    text = delete_items(text)

    text = replace_text_line(text)

    text = delete_num(text)

    text = replace_to_newline(text)

    text = replace_percent(text)

    text = replace_multilines(text, 20)

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

    delete_duplicates("./after.txt", "./after_del.txt")

    print("请手动对 ./after_del.txt 去重排版")


if __name__ == "__main__":
    main()
