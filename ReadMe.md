# 原神词库

这是为 rime 输入法（小狼毫｜鼠须管）编写的一个**未经特别详细筛选**的原神词库。基本是纯文本格式，支持weasel，也支持fcitx5。

## 包含内容

- 角色
  - 名称
  - 技能/天赋（及效果）
  - 命座
- 武器
  - 武器名
  - 武器特效
- 基本常识
  - 游戏教程中出现的生僻词（古月源珠供能基站、古月波纹等）
  - 地名（幕形态黑体物质笼实验室等）
  - 元素/元素反应（绽放类反应、月反应、星烁反应等）
  - 个人惯用名（凝冰渡海真君、椰羊、染水、扩冰等）
- NPC名（可能缺失最新版本的非关键NPC）
- 任务名（部分限时活动中的小任务可能未收录）
- 物品
  - 培养素材
  - 食物
  - 矿物
  - 掉落物
- 圣遗物（及效果）
- 成就（及成就描述中的关键词）
- 书籍
  - 书籍名称（可能缺失非关键书）
  - 书籍中的关键词（大量缺失）
- 等

## 更新方式：手动更新

1. 将 [Snap.Metadata](https://github.com/DGP-Studio/Snap.Metadata) 克隆到本地
   - 示例：`git clone https://github.com/DGP-Studio/Snap.Metadata --depth=5`
   - 添加安全目录 `git config --global --add safe.directory /path/to/Snap.Metadata`
2. 根据上版本的更新，选择性 diff 增量内容并输出到指定文件
   - 示例：`git diff 6a4fb98 main --minimal -- "*CHS*" ":!**/BeyondItem.json" ":!**/AgentSkill/**" > new.diff`
3. 正则筛选，运行 `regex.py` 文件，打开生成的 `./after_del.txt`
4. 手动断句
5. 运行 `merge.py`，它会依次执行：
   - 将 `after_del.txt` 追加到 `yuanshen.dict.yaml`
   - 调用 `genshin_pinyin_sort.exe` 按拼音排序并去除完全相同的重复行
   - 调用 `genshin_pinyin_add.exe` 生成 Fcitx5 词库 `yuanshen_fcitx5.txt`
6. 上传 `yuanshen.dict.yaml`、`yuanshen_fcitx5.txt`

## 致谢

感谢**以下项目**提供的数据：

- [Yap](https://github.com/Alex-Beng/Yap) - @Alex-Beng
- [BetterGI](https://github.com/babalae/better-genshin-impact/) - @babalae
- [Snap.Metadata](https://github.com/DGP-Studio/Snap.Metadata) - @DGP-Studio
- 我自己的经验
