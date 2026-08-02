# 项目说明

## 用途

本程序读取 Rime 的 `yuanshen.dict.yaml`，生成 Fcitx5 使用的 `yuanshen_fcitx5.txt`。它是 `Genshin_Words` 词库流水线的一环：把词典中的中文词条展开为全部合法拼音组合，并以 Tab 分隔词条输出给 Fcitx5 输入法。

## 词条格式

- Rime 手工标注词条必须使用三个 Tab 分隔字段：`词条<Tab>拼音<Tab>权重`。
- Rime 拼音字段的音节必须用单个空格分隔，例如：`薄缘的道与光与胤<Tab>bao yuan de dao yu guang yu yin<Tab>100`。
- 生成 Fcitx5 词库时，必须将拼音音节改为单引号分隔，例如：`bao'yuan'de'dao'yu'guang'yu'yin`。
- 自动生成和手工标注词条的权重均使用非负整数；自动生成词条默认权重为 `100`。
- 不得把 Rime 源词典中的空格分隔形式直接输出到 Fcitx5 词库，也不得把 Fcitx5 的单引号分隔形式写回 Rime 源词典。

## 多音字展开规则

- 每个字符通过 `pinyin` crate 的 `to_pinyin_multi()` 取读音，只保留前两个常用读音（`COMMON_READINGS_PER_CHARACTER = 2`），再排序去重，最后对逐字读音做笛卡尔积展开。
- 单个词条字符数上限 `MAX_CHARACTERS_PER_WORD = 256`，组合数上限 `MAX_COMBINATIONS_PER_WORD = 1_000_000`，超出即报错，不得静默截断。
- 拼音字符串零拷贝借用自 pinyin 数据，不产生额外分配。
- 去重由流水线前序步骤（`genshin_pinyin_sort` 的整行去重）保证；本程序不负责跨行去重。

## 构建与运行

```powershell
cargo build                # debug 构建
cargo build --release      # release 构建（strip、LTO、panic=abort）

.\genshin_pinyin_add.exe .\yuanshen.dict.yaml   # 唯一参数为输入文件路径
```

## 流水线上下文

`Genshin_Words` 词库的完整处理流程：

| 步骤 | 工具                           | 作用                                                 |
| ---- | ------------------------------ | ---------------------------------------------------- |
| 1    | `regex.py` + `after_del.txt`   | 从 `new.diff` 正则筛选增量内容，手动断句             |
| 2    | `merge.py`                     | 追加新词条到 `yuanshen.dict.yaml`，随后调用步骤 3、4 |
| 3    | `genshin_pinyin_sort`          | 按拼音排序并去除完全相同的重复行                     |
| 4    | `genshin_pinyin_add`（本程序） | 生成拼音组合并输出 Fcitx5 词库                       |

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

## 发布

推送 tag 触发 `.github/workflows/release.yml`，创建 GitHub Release 并附带 `yuanshen.dict.yaml` 与 `yuanshen_fcitx5.txt`；支持 `workflow_dispatch` 手动重新发布。
