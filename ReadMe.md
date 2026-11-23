# 原神词库

为 Rime（小狼毫、鼠须管）和 Fcitx5 提供的原神中文词库。收录角色、NPC、地点、团体、技能、天赋、命之座、武器、圣遗物、材料、任务、成就、书籍和家具等名称，以及部分玩家惯用词。不收录千星奇域内容。

## 使用词库

- Rime：`yuanshen.dict.yaml`。
- Fcitx5：`yuanshen_fcitx5.txt`，按拼音生成，词组内音节以单引号分隔。

两个文件从同一份 Rime 源词库生成。手工注音与多音词的不同读音分别保留。

## 每版本更新

需要 Python 3.12 及以上、Git、本项目的两个拼音程序，以及本地数据源：

| 项目 | 用途 | 默认位置 |
| --- | --- | --- |
| Snap.Metadata | 角色、技能、物品、成就和家具等中文字段 | `../Snap.Metadata` |
| Genshin_NPC_DATA | 导出 NPC、地点、称号、任务等带分类的词源 | `../Genshin_NPC_DATA` |
| GenshinData | NPC 提取程序读取的原始表与 TextMap | `../animegamedata2` |

先更新两个数据仓库，再在本项目目录运行批处理。四个提交号分别对应两个数据仓库的上个版本和本次版本：

```powershell
python update_dictionary.py <旧Metadata提交> <新Metadata提交> --npc-before <旧GenshinData提交> --npc-after <新GenshinData提交>
```

本地已经验证过的例子：

```powershell
python update_dictionary.py 5e1637c5169f d9252d4f44c7 --npc-before c5ef717b7741 --npc-after d54f6f0d423d
```

脚本依次构建 NPC 词源、提取版本增量、刷新保护表、纠错、按词汇拆长词、合并家具共有片段并过滤常用词。默认生成预览，正式词库保持原样。

结果位于 `.cache/update/`：

| 文件 | 内容 |
| --- | --- |
| `summary.md` | 数量统计和结果入口 |
| `candidates.txt` | 本批原始候选，可反复重放 |
| `new_texts.jsonl` | 新增原文、类别和字段位置 |
| `retained.json` | 保留的新词及来源 |
| `changes.json` | 每条删除、纠错和拆分的明细 |
| `furniture_families.json` | 家具共有部分及差异部分 |
| `review.json` | 无法自动确定的分词边界或手工读音问题 |
| `preview.dict.yaml` | 整理结果预览，不含待复核项 |
| `protected_terms.json` | 本批使用的分类保护表 |
| `proposed_rules.json` | 包含自动长词边界的规则预览 |

查看 `summary.md` 与 `review.json`。需要调整时，将决定写入 `curation_rules.json`，然后重放：

```powershell
python update_dictionary.py --candidates .cache/update/candidates.txt
```

确认结果后应用：

```powershell
python update_dictionary.py --candidates .cache/update/candidates.txt --apply
```

存在待复核项时，`--apply` 会停止并列出问题。没有待复核项时，先在临时目录生成并排序 Rime、Fcitx5，两份输出都成功后更新正式词库，同时保存保护表和长词分段。之后提交词库、规则和保护表。

常用路径参数：`--metadata`、`--npc-project`、`--npc-repo`、`--npc-sources`、`--output`。如果 NPC 词源已经生成，可以省略 `--npc-before` 和 `--npc-after`；此时使用现有 `term_sources.jsonl`，并将本项目人工维护的 `final_npc.txt` 作为补充词源。重放时默认读取候选文件同目录的保护表，也可用 `--sources` 指定。

## 整理规则

规则顺序为：精确纠错、超过十字的强制拆分、明确指定的拆分、专名保护、家具系列拆分、常用词与组合去重。

成就名称与详情、技能名称与特效、角色名称与归属、命之座、天赋、武器名称与特效、圣遗物、天赋书，以及 NPC、地点、团体等专名按来源保护。超过十字时按词汇拆开，优先保留其中完整的专名。未知来源的组合词保留。

家具允许拆分。程序从全部家具数据中寻找共有前缀和后缀：有“的”这类明确边界时至少需要两条同系列名称；无明确边界时至少需要三条，并检查剩余部分的词汇边界。图纸介绍中重复引用家具名，不会将它重新设为专名。

例如：

| 原词 | 处理结果 |
| --- | --- |
| 阿赫玛尔的故事 | 按明确规则拆开，保留“阿赫玛尔” |
| 阿赫玛尔之须、阿赫玛尔之眼 | 保留整词 |
| 慈爱的教导、慈爱的指引、慈爱的哲学、慈爱的淑女帽 | 天赋书和圣遗物，保留整词 |
| 基地的宝库大门、基地的沉静漫射 | “基地”“的”由基础词库提供，保留“宝库大门”“沉静漫射” |
| 智慧之城的丝缕、智慧之城的兼罗 | 共有部分“智慧之城”只收录一次，再保留差异部分 |

常用词依据固定版本的词频表判断，默认门槛为 500；标为人名、地名、团体名或其他专名的词不作为常用词删除。词频来源和许可证见 [data/README.md](data/README.md)。

在 `curation_rules.json` 中保存人工决定：

| 字段 | 用途 |
| --- | --- |
| `keep` | 整词保留 |
| `common_words` | 补充容易输入的普通词 |
| `composable` | 允许尝试组合去重的已核对短语 |
| `split` | 指定分词边界，分段必须逐字拼回原词且每段最多十字 |
| `corrections` | 错词、正确词和原始文本证据 |
| `exclude` | 经核对的提取残片，记录排除原因和原文证据 |
| `common_min_frequency` | 常用词频门槛 |
| `furniture_min_family` | 家具系列所需的最少名称数，默认 2 |

手工修订会用于后续版本。纠错采用完整词精确替换，不依据近形或近音自动改字；带手工拼音的错词必须同时指定新拼音列。

家具的人工拆分边界也会用于自动系列拆分：已修正的片段不会在重新导入完整家具名时再次生成。摆设图纸和摆设套装图纸都按家具处理。

## 整理已有词库

预览：

```powershell
python curate_dictionary.py
```

应用并同步两个输入法格式：

```powershell
python merge.py --no-append
```

预览明细在 `.cache/curation_report.json`，合并明细在 `.cache/merge_report.json`。`curate_dictionary.py --write` 只更新 Rime 源文件。需要重建保护表时加 `--refresh-sources`。

仍可单独运行 `extract_words.py <旧提交> <新提交>`，审校 `after_del.txt` 后运行 `merge.py`。省略提交号时使用下列区间：

`git diff 5e1637c5169f..d9252d4f44c7`

历史快照保存在 `.snapshots/<完整提交号>/`；修改文字处理规则后可复用，强制重新解包使用 `--rebuild`。

本次清理的逐词记录见 [整理报告](reports/curation-2026-11-23.md) 和 [JSON 明细](reports/curation-2026-11-23.json)。

后续逐行重审见 [重审报告](reports/reaudit-2026-11-23.md)、[逐行记录](reports/reaudit-2026-11-23.tsv) 和 [变更证据](reports/reaudit-2026-11-23.json)。

## 拼音配置与验证

`pinyin_rules.json` 维护字级、词级读音白名单及字级黑名单。生成程序按雾凇拼音字音权重筛选读音，默认门槛为每字最高权重的 0.1%，每字最多保留两个读音。规则格式、重建方式和数据来源见 [genshin_pinyin_add/README.md](genshin_pinyin_add/README.md)。

```powershell
python -m unittest discover -s tests -v
```

测试覆盖来源分类、家具系列拆分、专名保护、长词边界、纠错、手工注音、批次复核和生成失败时的文件保留。

## 数据来源

感谢 Yap、BetterGI、Snap.Metadata、GenshinData 及相关维护者提供的数据。词库同时包含人工收录与修订的内容。
