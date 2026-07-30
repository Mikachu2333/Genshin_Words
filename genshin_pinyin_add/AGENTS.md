# 项目说明

## 用途

本程序读取 Rime 的 `yuanshen.dict.yaml`，生成 Fcitx5 使用的 `yuanshen_fcitx5.txt`。

## 词条格式

- Rime 手工标注词条必须使用三个 Tab 分隔字段：`词条<Tab>拼音<Tab>权重`。
- Rime 拼音字段的音节必须用单个空格分隔，例如：`薄缘的道与光与胤<Tab>bao yuan de dao yu guang yu yin<Tab>100`。
- 生成 Fcitx5 词库时，必须将拼音音节改为单引号分隔，例如：`bao'yuan'de'dao'yu'guang'yu'yin`。
- 自动生成和手工标注词条的权重均使用非负整数；自动生成词条默认权重为 `100`。
- 不得把 Rime 源词典中的空格分隔形式直接输出到 Fcitx5 词库，也不得把 Fcitx5 的单引号分隔形式写回 Rime 源词典。

## 修改要求

- 保留 YAML 头部解析、输入大小限制、组合数量限制和原子写入机制。
- 新增或修改格式处理时，必须同时覆盖手工标注词条与自动生成词条的测试。
- 错误信息应写入 stderr，正常处理结果写入 stdout。
- 禁止使用 `unsafe`，保持 Rust 2024 与 `cargo clippy` 兼容。

## 验证

```powershell
$ErrorActionPreference = 'Stop'
cargo fmt --check
cargo test
cargo clippy --all-targets -- -D warnings
```
