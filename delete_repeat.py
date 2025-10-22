from collections import OrderedDict


def delete_duplicates(input_file_path: str, output_file_path: str):
    """
    使用OrderedDict高效删除文件中的重复行，保持原有顺序

    Args:
        input_file_path: 输入文件路径
        output_file_path: 输出文件路径，如果为None则覆盖原文件
    """
    try:
        with open(input_file_path, "r", encoding="utf-8") as file:
            lines = file.readlines()

        # 使用OrderedDict.fromkeys()去重，这是最高效的方法之一
        unique_lines = list(OrderedDict.fromkeys(
            line.rstrip("\n\r") for line in lines))

        # 确定输出文件路径
        if output_file_path == "":
            output_file_path = input_file_path

        # 写入文件
        with open(output_file_path, "w", encoding="utf-8") as file:
            for line in unique_lines:
                file.write(line + "\n")

        print(f"去重完成！")
        print(f"原始行数: {len(lines)}")
        print(f"去重后行数: {len(unique_lines)}")
        print(f"删除重复行数: {len(lines) - len(unique_lines)}")

        return True

    except Exception as e:
        print(f"处理文件时发生错误: {str(e)}")
        return False
