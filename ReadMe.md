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
  - 尘歌壶（家具与套装）
- 圣遗物（及效果）
- 成就（及成就描述中的关键词）
- 书籍
  - 书籍名称（可能缺失非关键书）
  - 书籍中的关键词（大量缺失）
- 等

**不支持千星奇域的相关内容。**

## 更新方式：手动更新

1. 将 [Snap.Metadata](https://github.com/DGP-Studio/Snap.Metadata) 克隆到本地
   - 示例：`git clone https://github.com/DGP-Studio/Snap.Metadata --depth=5`
   - `--depth` 必须覆盖到上一个版本，`extract_words.py` 要读两个 commit 的完整 JSON
   - 添加安全目录 `git config --global --add safe.directory /path/to/Snap.Metadata`
2. 把下面这行的 commit 区间换成本次要对比的两个版本，`extract_words.py` 就按它取数
   - 示例：`git diff 56e97b214a7e..5e1637c5169f`
   - 也可以直接写参数：`python extract_words.py 56e97b2 5e1637c`
   - 两个 commit 的 `Genshin/CHS` 快照会被解包到 `.snapshots/<完整 hash>/`，重复运行只解包一次，
     要强制重解包加 `--rebuild`
3. 运行 `extract_words.py`：它按 `text_fields.py` 里的字段表逐文件取文本，用旧快照的全部文本做差集，
   把**真正新增**的文本与手工维护的 `final_npc.txt` 一起走正则管线、去重，输出 `./after_del.txt`
   - 取哪一层、哪些字段只在 `text_fields.py` 里改；没列出的文件不参与取词（图标名、时间、版本号、
     内部 ID 这类字段本来就不该进词典）
   - 用差集而不是 `git diff`：内容没变只是换了顺序、或从别的字段搬过来的旧文本不再被当成新增
   - 每条文本都是完整的字段值，不会再被 diff 的行边界切碎
   - `.cache/new_texts.jsonl` 保留差集得到的原始文本（一行一条 JSON），调整正则时可以拿它反复试，
     不必重新解包；`--metadata` 可以指向别的 `Snap.Metadata` 克隆
4. 人工审校 `after_del.txt`：纯汉字过滤已经做完，剩下的只是决定某个词要不要收
5. 运行 `merge.py`，它会依次执行：
   - 将 `after_del.txt` 追加到 `yuanshen.dict.yaml`
   - 调用 `genshin_pinyin_sort.exe` 按拼音排序并去除完全相同的重复行
   - 调用 `genshin_pinyin_add.exe` 生成 Fcitx5 词库 `yuanshen_fcitx5.txt`
   - 重复跑不会让词典越滚越大，整行去重会把重复合并的旧词条收掉
6. 上传 `yuanshen.dict.yaml`、`yuanshen_fcitx5.txt`

## 致谢

感谢**以下项目**提供的数据：

- [Yap](https://github.com/Alex-Beng/Yap) - @Alex-Beng
- [BetterGI](https://github.com/babalae/better-genshin-impact/) - @babalae
- [Snap.Metadata](https://github.com/DGP-Studio/Snap.Metadata) - @DGP-Studio
- 我自己的经验
