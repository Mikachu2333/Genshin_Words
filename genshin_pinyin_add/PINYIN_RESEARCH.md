# 拼音仓库调研与实现结果

调研日期：2026-11-22。近六个月窗口：2026-05-22 至 2026-11-22。更新判断使用实际代码或词典修改，星数为调研时的约数。

## 仓库比较

| 仓库 | 星数 | 窗口内实质更新证据 | 多音字及排序依据 | 适用性 |
| --- | ---: | --- | --- | --- |
| [iDvel/rime-ice](https://github.com/iDvel/rime-ice) | 20,300 | [2026-11-11 增改中文词汇](https://github.com/iDvel/rime-ice/commit/927e1cf1d3eeb9bf4be85e3f684dba4dd485b6f6)，3 个词典文件、415 行新增、326 行删除 | 字表包含逐字多音及数值权重，Rime 配置 `sort: by_weight`。权重来自 25 亿字字频，并对各音作人工调整 | 采用其字音权重表，按每字权重降序筛选，适合常用读音派生 |
| [mozillazg/pinyin-data](https://github.com/mozillazg/pinyin-data) | 1,500 | [2026-10-24 修正没、炔、唵等读音](https://github.com/mozillazg/pinyin-data/commit/0051cab569706cc814a4cffaf58281a1de5481b6)；[2026-11-01 更新并重生成数据](https://github.com/mozillazg/pinyin-data/commit/c58ae7d5e178a8424e0d3e8622c47c3f8420200e) | 全量多音表加 Unihan `kHanyuPinlu`；合并的 `pinyin.txt` 提供首选读音，全列表缺少逐音数值频次。原始 Unihan 频次可另行提取 | 真实语料频次的备选来源，但频次覆盖较窄 |
| [mozillazg/python-pinyin](https://github.com/mozillazg/python-pinyin) | 5,400 | [2026-11-01 修复 ng 韵母](https://github.com/mozillazg/python-pinyin/commit/b35b2472981c1d4f6fa9bd464d73f2afd84da1d0)；[2026-10-30 修复词语分词后缀重扫](https://github.com/mozillazg/python-pinyin/commit/f51645c65caeda555a0bf524367762df0cbfb24f) | 支持 heteronym，使用 pinyin-data，常规 API 返回读音集合，未提供逐音频次值 | 可用于词语消歧；派生阈值仍需外部权重数据 |
| [zh-lx/pinyin-pro](https://github.com/zh-lx/pinyin-pro) | 4,800 | [2026-10-09 三声变调可选配置](https://github.com/zh-lx/pinyin-pro/commit/a7846e730913faf74b89b3c89ac817020dcb2ce4)；[2026-10-04 自定义拼音批量更新优化](https://github.com/zh-lx/pinyin-pro/commit/d33c8c29d3862201f36ad51754608b98b3614b91) | 多音表含 `什: shén shí`、`长: cháng zhǎng`、`哪: nǎ na nǎi né něi`，没有每音频次或频率值 | 首音及集合可用，读音列表顺序不足以实现数值阈值 |
| [hotoo/pinyin](https://github.com/hotoo/pinyin) | 7,800 | [2026-06-09 修正“乡长”](https://github.com/hotoo/pinyin/commit/6580657e327edd4a94bd87325ce4d47dc2409126) | 支持多音字；[常用字表](https://github.com/hotoo/pinyin/blob/6580657e327edd4a94bd87325ce4d47dc2409126/packages/tools/dict/dict-zi-frequent.js) 的“什”为 `shí,shén`，未按要求把 shen 放在 shi 前，也没有每音数值频次 | 不符合本次要求的频率排序依据 |
| [mozillazg/rust-pinyin](https://github.com/mozillazg/rust-pinyin) | 240 | 最后推送为 2026-05-12，早于窗口起点 | 当前 Rust crate 提供多音列表及首选读音，无逐音数值频次 | 保留为缺字首选读音的回退；未满足维护时间要求 |

本次采用雾凇的**字音权重**作为常用程度指标。原始文件声明字频来源，并说明多音字的权重会为自动注音人工调整，因此数值应按输入法字音权重理解。其他广为使用的拼音转换库提供首音优先或多音集合，难以直接给出本次需要的可调数值阈值。

## Unihan 频次的核对

同时检查了 [Unicode 18.0.0 Unihan 数据](https://www.unicode.org/Public/18.0.0/ucd/Unihan.zip)中的 `kHanyuPinlu`。它来自《现代汉语频率词典》，提供实际读音出现次数；声调变体转为无声调音节后相加。

| 字 | 原始频次 | 无声调汇总 |
| --- | --- | --- |
| 长 | zhǎng(1879)、cháng(1179) | zhang=1879、chang=1179 |
| 什 | shén(4045)、shen(17) | shen=4062，shi 未在该频次字段列出 |
| 哪 | nǎ(1138)、na(266) | na=1404，ne 未在该频次字段列出 |
| 还 | hái(5894)、huán(103) | hai=5894、huan=103 |
| 着 | zhe(10643)、zháo(545)、zhuó(125)、zhāo(15) | zhe=10643、zhao=560、zhuo=125 |

该字段覆盖 3,799 个字，在当前词库的 3,817 个不同字符中覆盖 2,428 个；按字符出现次数覆盖 84,865 / 96,464，约 88%。它未提供“什/shi”等读音的数值频次。雾凇权重对常用备选音覆盖更充分，因此作为本次默认派生依据。

## 阈值与名单

默认常量为 `MIN_READING_FREQUENCY_PERCENT = 0.1`，条件为：

```text
本音权重 × 100 >= 本字最高权重 × 阈值百分比
```

阈值支持小数百分比，权重用 `u32` 保存并无损转为 `f64` 参与比较。先筛频率、过滤黑名单，再按权重降序取最多两个不同音节。0.1% 低于“什/shi”约 0.207% 和“还/huan”约 6% 的权重比例，又高于“莘/xin”约 0.0148% 和“哪/ne”约 0.0001% 的比例；“长”的两个音权重相同，均保留。5% 更适合严格限制自动注音，当前词库含大量“阿什…”等人名，默认采用 0.1% 保留 shi。

词级白名单和字级白名单指定完整读音集合，覆盖频率筛选和黑名单；词级规则只作用于匹配词段的字位，也会应用于手工注音词条。多个多音字可以同时指定，例如“阿房宫赋”分别指定“阿/e、房/pang”。默认 [pinyin_rules.json](../pinyin_rules.json) 为空，用户示例保存在测试夹具中。

## 排序检查与改进

原 `genshin_pinyin_sort` 已正确按整行去重、保留同音词与同词不同手工读音，但存在两处问题：

1. 按词语分组时，逐条线性查找已有组，最坏 O(n²)。改为 `HashMap` 为词语建立组编号，配合排序保持整体 O(n log n)，词语、读音、原行在排序过程中借用输入。
2. 自动词条的键含声调数字，手工词条的键去掉音节空格。例如自动“安宁”的 `an1ning2` 会排在手工“安康”的 `ankang` 前。两类词条现统一使用无声调拼音键。

分组规则继续保留：同词异音集中，词组位置由其最小拼音键决定，组内按拼音排列，同键用输入位置决定先后。手工 Rime 行保持原内容；整行重复由 Rime 入口去重。

原 `genshin_pinyin_add` 没有引用 `sort_lib`，只沿用源行顺序，并把 `pinyin` 的前两个带调读音当作常用读音。现已通过路径依赖复用 `sort_lib::sort_fcitx5_lines`，在派生完成后统一排序全部 Fcitx5 行；手工 Rime 三字段解析也复用共享库。

## 性能测量

同一 release 进程读取输入后，旧、新排序实现各执行 5 次，使用耗时中位数；测量包括校验、去重、排序和结果分配。

| 输入 | 改进前 | 改进后 |
| --- | ---: | ---: |
| 当前约 2.26 万行词库 | 718.9 ms | 12.4 ms |
| 5,000 个不同词条 | 51.1 ms | 2.2 ms |
| 10,000 个不同词条 | 175.3 ms | 4.6 ms |
| 20,000 个不同词条 | 673.1 ms | 9.4 ms |

当前词库约快 58 倍；增长曲线也显示已消除线性分组造成的平方级成本。`add` 为最终结果排序增加了输出缓冲，缓冲受 512 MiB 输出限额约束。

## 最终验证

排序库 14 项测试、排序程序 5 项测试、派生程序 33 项测试全部通过，三个包的格式与 Clippy 检查通过。使用完整词库另行计算读音组合及词组排序，与 release 程序输出逐行一致；手工拼音、原权重、黑白名单示例及原子写入失败处理均已核对。

已更新两个流水线可执行文件并重新生成 Fcitx5 词库。输出从 72,987 行、2,983,265 字节缩减为 32,106 行、1,191,322 字节；前后词语集合相同，共 22,614 个词语。默认黑白名单为空，用户示例保存在测试夹具中。
