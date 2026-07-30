# 项目说明

## 用途

本程序对 Rime 的 `yuanshen.dict.yaml` 正文排序，更新版本号和时间，并安全替换原文件。

## 词条格式

- 普通词条可以只包含词语，由程序根据首选拼音生成排序键。
- 手工标注词条必须使用三个 Tab 分隔字段：`词条<Tab>拼音<Tab>权重`。
- 手工拼音的音节必须用单个空格分隔，例如：`薄缘的道与光与胤<Tab>bao yuan de dao yu guang yu yin<Tab>100`。
- 权重必须是非负整数；项目约定新增手工词条通常使用 `100`。
- 排序时应忽略拼音字段中的音节空格，但输出时必须原样保留整条手工词条。
- 不得将 Rime 源词典改成 Fcitx5 使用的单引号分隔格式。

## 修改要求

- 保持固定六行 YAML 头部，结束标记必须为 `...`，排序配置必须为 `sort: original`。
- 同音词和同一词条的多个手工读音不得因排序而丢失。
- 保留输入/输出大小限制、中间文件及安全交换机制。
- 修改排序或格式校验时，必须增加相应测试。
- 禁止使用 `unsafe`，保持 Rust 2024 与 `cargo clippy` 兼容。

## 验证

```powershell
$ErrorActionPreference = 'Stop'
cargo fmt --check
cargo test
cargo clippy --all-targets -- -D warnings
```
