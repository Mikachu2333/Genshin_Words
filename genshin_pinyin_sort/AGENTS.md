# 项目说明

## 用途

本程序对 Rime 的 `yuanshen.dict.yaml` 正文排序、去除完全相同的重复行、更新版本号和时间，并安全替换原文件。

## 词条格式

- 普通词条可以只包含词语，由程序根据首选拼音生成排序键。
- 手工标注词条必须使用三个 Tab 分隔字段：`词条<Tab>拼音<Tab>权重`。
- 手工拼音的音节必须用单个空格分隔，例如：`薄缘的道与光与胤<Tab>bao yuan de dao yu guang yu yin<Tab>100`。
- 权重必须是非负整数；项目约定新增手工词条通常使用 `100`。
- 排序时应忽略拼音字段中的音节空格，但输出时必须原样保留整条手工词条。
- 自动与手工词条统一使用无声调拼音排序键；不得把声调数字混入自动词条的排序键。
- 不得将 Rime 源词典改成 Fcitx5 使用的单引号分隔格式。

## 共享排序

- `sort_lib::sort_chinese_text` 处理 Rime 源词典，执行整行去重。
- `sort_lib::sort_fcitx5_lines` 处理 Fcitx5 三字段词条，供 `genshin_pinyin_add` 在派生完成后调用，保留相同输出行。
- 两个入口共用排序实现：先按无声调拼音建立词组顺序，再按词语分组、组内按拼音排序，同键以输入位置决定顺序。
- 分组使用 `HashMap` 建立组编号，保持整体 O(n log n)，避免逐条线性查找已有组。

## 去重规则

- 去重只针对完全相同的整行（trim 后），保留首次出现的行；同音词、同一词条的不同手工读音（如 `bao` 与 `bo` 两条）内容不同，不会被合并。
- 去重键必须是整行内容，不得改用拼音排序键或词条名，否则会误删同音词或同一词条的多读音条目。
- 历史教训：早期实现用 `HashMap` 以拼音为键去重，导致同音词互相覆盖；改用 `Vec` 稳定排序后一度完全丢失去重；现按整行去重，两项语义同时成立。

## 修改要求

- 保持固定六行 YAML 头部，结束标记必须为 `...`，排序配置必须为 `sort: original`。
- 同音词和同一词条的多个手工读音不得因排序而丢失；完全相同的重复行必须被去重。
- 保留输入/输出大小限制、中间文件及安全交换机制。
- 修改排序、去重或格式校验时，必须增加相应测试。
- 禁止使用 `unsafe`，保持 Rust 2024 与 `cargo clippy` 兼容。

## 验证

```powershell
$ErrorActionPreference = 'Stop'
cargo fmt --check
cargo test
cargo clippy --all-targets -- -D warnings
# sort_lib 是独立包，必须另外验证其测试与 lint
cargo test --manifest-path sort_lib/Cargo.toml
cargo clippy --manifest-path sort_lib/Cargo.toml --all-targets -- -D warnings
```
