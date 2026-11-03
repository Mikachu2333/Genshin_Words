"""每个 JSON 文件从哪些字段取游戏文本。

键是相对 `Genshin/CHS` 的文件路径 glob，值是字段表达式：

- `Name`：根对象的 `Name` 字段
- `SkillDepot.Skills[].Name`：逐层下钻，`[]` 表示遍历该层的数组
- `*`：遍历该层对象的全部值（按 JSON 中的顺序）

同一文件可以列出多条表达式，按顺序取值。没有列出的文件不参与取词，
图标名、时间、版本号、内部 ID 这类字段本来就不该进词典。
改动这张表后需要重跑 `extract_words.py`；快照可以直接复用，不必加 `--rebuild`。
"""

# 角色文件的字段在 120 多个 ID 文件之间共用一份，单独抽出来
AVATAR_TEXT_FIELDS: tuple[str, ...] = (
    "Name",
    "Description",
    "FetterInfo.Title",
    "FetterInfo.Detail",
    "FetterInfo.ConstellationBefore",
    "FetterInfo.ConstellationAfter",
    "FetterInfo.VisionBefore",
    "FetterInfo.VisionAfter",
    "FetterInfo.VisionOverrideLocked",
    "FetterInfo.VisionOverrideUnlocked",
    "FetterInfo.Native",
    "FetterInfo.Fetters[].Title",
    "FetterInfo.Fetters[].Context",
    "FetterInfo.FetterStories[].Title",
    "FetterInfo.FetterStories[].Context",
    "Costumes[].Name",
    "Costumes[].Description",
    "NameCard.Name",
    "NameCard.Description",
    "SkillDepot.Skills[].Name",
    "SkillDepot.Skills[].Description",
    "SkillDepot.Skills[].SpecialDescription",
    "SkillDepot.Skills[].Proud.Descriptions[]",
    "SkillDepot.EnergySkill.Name",
    "SkillDepot.EnergySkill.Description",
    "SkillDepot.EnergySkill.SpecialDescription",
    "SkillDepot.EnergySkill.Proud.Descriptions[]",
    "SkillDepot.Inherents[].Name",
    "SkillDepot.Inherents[].Description",
    "SkillDepot.Inherents[].Proud.Descriptions[]",
    "SkillDepot.SpecialSkills[].Name",
    "SkillDepot.SpecialSkills[].Description",
    "SkillDepot.Talents[].Name",
    "SkillDepot.Talents[].Description",
    "SkillDepot.Talents[].SpecialDescription",
    "TraceEffect.Name",
    "TraceEffect.Description",
)

TEXT_FIELDS: dict[str, tuple[str, ...]] = {
    # 角色：名称、介绍、好感故事、命座、天赋技能、装扮、名片
    "Avatar/*.json": AVATAR_TEXT_FIELDS,
    # 成就与成就分类
    "Achievement.json": ("[].Title", "[].Description"),
    "AchievementGoal.json": ("[].Name",),
    # 章节
    "Chapter.json": ("[].Number", "[].Title", "[].ImageTitle"),
    # 合成配方
    "Combine.json": ("[].EffectDescription",),
    # 图鉴展示物（饮品等）
    "DisplayItem.json": ("[].Name", "[].Description", "[].TypeDescription"),
    # 尘歌壶家具、套装、分类
    "Furniture.json": ("[].Name", "[].Description", "[].SourceTexts[]"),
    "FurnitureSuite.json": ("[].Name", "[].Description"),
    "FurnitureType.json": ("[].Name", "[].Name2"),
    # 卡池
    "GachaEvent.json": ("[].Name",),
    # 高难挑战排期
    "HardChallengeSchedule.json": ("[].Name",),
    # 超链接词
    "HyperLinkName.json": ("[].Name", "[].Description"),
    # 材料、怪物
    "Material.json": (
        "[].Name",
        "[].Description",
        "[].TypeDescription",
        "[].EffectDescription",
    ),
    "Monster.json": ("[].Name", "[].Title", "[].Description", "[].Affixes[]"),
    # 名片、头像
    "NameCard.json": ("[].Name", "[].Description", "[].SpecialDescription"),
    "ProfilePicture.json": ("[].Name",),
    # 圣遗物
    "Reliquary.json": ("[].Name", "[].Description"),
    "ReliquarySet.json": ("[].Name", "[].Descriptions[]"),
    # 深境螺旋
    "TowerFloor.json": (
        "[].Descriptions[]",
        "[].FirstDescriptions[]",
        "[].SecondDescriptions[]",
    ),
    "TowerLevel.json": (
        "[].FirstGadget.Name",
        "[].FirstGadget.Description",
        "[].SecondGadget.Name",
        "[].SecondGadget.Description",
        "[].FirstWaves[].Description",
        "[].FirstWaves[].Monsters[].Affixes[].Name",
        "[].FirstWaves[].Monsters[].Affixes[].Description",
        "[].SecondWaves[].Description",
        "[].SecondWaves[].Monsters[].Affixes[].Name",
        "[].SecondWaves[].Monsters[].Affixes[].Description",
    ),
    "TowerSchedule.json": ("[].BuffName", "[].Descriptions[]"),
    # 武器
    "Weapon.json": (
        "[].Name",
        "[].Description",
        "[].Affix.Name",
        "[].Affix.Descriptions[].Description",
    ),
}
