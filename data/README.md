# 整理词库使用的数据

`jieba.dict.txt` 来自 jieba v0.42.1 的 `jieba/dict.txt`，保留原始字节。

- 来源：`https://github.com/fxsjy/jieba/blob/v0.42.1/jieba/dict.txt`
- SHA-256：`7197c3211ddd98962b036cdf40324d1ea2bfaa12bd028e68faa70111a88e12a8`
- 许可证：MIT，见 `LICENSE.jieba`。

程序只读其中的词频和词性，使用 Python 标准库分词，不需要安装 jieba。默认只有词频至少 500 且未标为人名、地名、团体名或其他专名的多字词才可作为常用词依据；人工确认的常用词在 `curation_rules.json` 的 `common_words` 中维护。

`protected_terms.json` 由本地 Snap.Metadata 和 Genshin_NPC_DATA 的分类来源生成。文件头记录元数据提交、NPC 数据提交和来源表 SHA-256。`terms` 保存专名及其定位证据，`furniture` 保存可以组合拆分的家具名称；同词同时出现在两类中时，专名保护优先。每词最多保留三条来源定位。

这个文件随仓库提交，日常整理可离线运行。更新游戏版本时运行 `extract_words.py` 会用新快照重建它，也可以单独运行：

```powershell
python curate_dictionary.py --refresh-sources
```

源项目路径可用 `--metadata`、`--npc-sources` 指定。家具描述中的引号专名依然可以受到保护。

家具名称的重复引用不进入专名表，避免图纸描述把家具重新锁定。家具与成就、技能等受保护字段同名时仍保留整词。批处理入口 `update_dictionary.py` 将保护表写在本批预览目录中，应用后再更新本文件。

`Material.json` 中的“摆设图纸”和“摆设套装图纸”名称均归入家具类。
