# coding=utf-8
from pathlib import Path
import json
import io


def data_return(file: Path):
    """返回json格式的数据"""
    f = file.read_text(encoding="utf8")
    return json.loads(f)


def json_data_output(raw_data: list, need_list: list, text_writer: io.TextIOWrapper):
    """输出需要的内容到text文件"""
    for i in raw_data:
        for j in need_list:
            if j in i:
                if isinstance(i[j], list):
                    for k in i[j]:
                        text_writer.write(k + "\n")
                else:
                    text_writer.write(i[j] + "\n")
"""
角色的输出
    for i in raw_data:
        if isinstance(i, list) or isinstance(i, dict):  # i是否为list或dict
            for j in i["Costumes"]:  # 衣装
                text_writer.write(j["Name"] + "\n")
            j = i["FetterInfo"]
            try:  # 命座名称
                k = j["ConstellationAfter"]
                text_writer.write(j["ConstellationAfter"] + "\n")
            except:
                text_writer.write(j["ConstellationBefore"] + "\n")
            text_writer.write(j["Native"] + "\n")  # 归属
            text_writer.write(j["Title"] + "\n")  # 卡池名
            j = i["SkillDepot"]
            text_writer.write(j["EnergySkill"]["Name"] + "\n")  # q
            for k in j["Inherents"]:
                text_writer.write(k["Name"] + "\n")  # 附加天赋
            for k in j["Skills"]:
                text_writer.write(k["Name"] + "\n")  # ae...
            for k in j["Talents"]:
                text_writer.write(k["Name"] + "\n")  # 命座

"""

def data_read(file_name: str, file_data: list, text: io.TextIOWrapper):
    """根据不同文件分别读取不同内容并调用json_data_output输出到文件"""
    print(file_name)
    match file_name:
        case "Achievement":  # 成就
            return json_data_output(file_data, ["Title", "Description"], text)
        case "AchievementGoal":  # 成就大类
            return json_data_output(file_data, ["Name"], text)
        case "Avatar":  # 角色
            return json_data_output(file_data, ["Name", "Description"], text)
        case "DisplayItem":  # 物品
            return json_data_output(
                file_data, ["Name", "Description", "TypeDescription"], text
            )
        case "Furniture":  # 家具
            return json_data_output(file_data, ["Name", "Description"], text)
        case "FurnitureSuite":  # 家具套装描述
            return json_data_output(file_data, ["Name", "Description"], text)
        case "FurnitureType":  # 家具类别
            return json_data_output(file_data, ["Name", "Name2"], text)
        case "GachaEvent":  # 祈愿名称
            return json_data_output(file_data, ["Name"], text)
        case "Material":  # 食物武器圣遗物套装天赋
            return json_data_output(
                file_data, ["Name", "Description", "TypeDescription"], text
            )
        case "Monster":  # 怪物
            return json_data_output(file_data, ["Name", "Title", "Description"], text)
        case "ProfilePicture":  # 衣装
            return json_data_output(file_data, ["Name"], text)
        case "Reliquary":  # 圣遗物详情
            return json_data_output(file_data, ["Name", "Description"], text)
        case "ReliquarySet":  # 圣遗物套装效果
            return json_data_output(file_data, ["Name", "Descriptions"], text)
        case "TowerFloor":  # 副本buff
            return json_data_output(file_data, ["Descriptions"], text)
        case "TowerSchedule":  # 深渊buff
            return json_data_output(file_data, ["BuffName", "Descriptions"], text)
        case "Weapon":  # 武器
            return json_data_output(file_data, ["Name", "Description"], text)
        case _:
            return None


# 主文件
p = Path.cwd().resolve()
print(p)
raw_data_path = p.joinpath("raw")
text_path = p.joinpath("test.txt")

text_data = io.open(text_path, "w+", encoding="utf8")

result = list(raw_data_path.iterdir())
dont_read = {
    "AvatarCurve",
    "AvatarPromote",
    "FurnitureMake",
    "Meta",
    "MonsterCurve",
    "ReliquaryAffixWeight",
    "ReliquaryMainAffix",
    "ReliquaryMainAffixLevel",
    "ReliquarySubAffix",
    "TowerLevel",
    "WeaponCurve",
    "WeaponPromote",
    "ReadMe"
}
for i in result:
    if i.stem not in dont_read:
        context = data_return(i)
        data_read(i.stem, context, text_data)

text_data.close()
