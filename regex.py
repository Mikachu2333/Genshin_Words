import os
import random
import string

from replace_self import *
from delete_repeat import delete_duplicates


def process_text(text: str):
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

    text = delete_none_chinese(text)
    text = replace_to_newline(text)

    text = replace_multilines(text, 20)

    return text


def generate_random_filename_chars(length: int = 8, extension: str = ''):
    """
    使用随机字符生成文件名

    Args:
        length (int): 文件名长度
        extension (str): 文件扩展名

    Returns:
        str: 随机文件名
    """
    chars: str = string.ascii_lowercase + string.digits
    filename = ''.join(random.choice(chars) for _ in range(length))
    if extension:
        filename += extension
    return filename


def main():
    """
    主函数，处理用户输入的文件
    """

    # 使用前修改
    file_path = "./new.diff"
    final_path = "./after_del.txt"

    # 生成输出文件名
    temp_path = generate_random_filename_chars(extension=".txt")
    try:
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

        # 保存处理后的文本
        with open(temp_path, "w", encoding="utf-8") as file:
            file.write(processed_text)

        print(f"处理完成")

    except Exception as e:
        print(f"处理文件时发生错误: {str(e)}")

    delete_duplicates(temp_path, final_path)
    os.remove(temp_path)
    print("请手动对 ./after_del.txt 去重排版")


if __name__ == "__main__":
    main()
