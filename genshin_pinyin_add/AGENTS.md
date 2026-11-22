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

- 自动词条使用内嵌的雾凇拼音字音权重表，按每字各读音的权重降序选择；阈值 `MIN_READING_FREQUENCY_PERCENT = 0.1` 表示该字最高权重的百分比，支持小数百分比，通过筛选后最多保留 `COMMON_READINGS_PER_CHARACTER = 2` 个不同的无声调音节。
- 权重表缺失的字使用 `pinyin` crate 的首选读音；替代读音通过白名单指定。
- `pinyin_rules.json` 默认从输入词典所在目录读取，也可通过第二个命令行参数指定；当前名单由用户手动维护，示例独立存放于测试夹具。
- 名单优先级为词级白名单、字级白名单、黑名单、自动读音。词级白名单每字用一个字符串指定读音；字级白名单单读音用字符串，双读音用固定两项数组。白名单指定匹配位置的完整读音集合；词级白名单仅影响命中词段中的字，并支持同时指定多个字、多个匹配和重叠匹配，较长词段优先，同等长度的冲突规则报错。
- 黑名单只包含 `characters`，每字用一个字符串指定禁止读音。自动词条先取最多两个候选，再应用黑名单，删除后不补入第三个读音；过滤后没有读音则报错。名单对手工 Rime 词条同样生效；未命中的字位沿用手工拼音，原权重保留，重组要求一字对应一个音节。
- 单个词条字符数上限 `MAX_CHARACTERS_PER_WORD = 256`，组合数上限 `MAX_COMBINATIONS_PER_WORD = 1_000_000`，超出即报错，不得静默截断。
- 拼音音节借用自内嵌字音表、`pinyin` 数据或名单配置，最终组合字符串复用缓冲区。
- 派生结果在 `MAX_OUTPUT_BYTES` 限额内缓冲，随后调用 `sort_lib::sort_fcitx5_lines`，保证手工词条和自动词条共用排序算法、同词异音集中。
- 去重由流水线前序步骤（`genshin_pinyin_sort` 的整行去重）保证；本程序不负责跨行去重。

## 构建与运行

```powershell
cargo build                # debug 构建
cargo build --release      # release 构建（strip、LTO、panic=abort）

.\genshin_pinyin_add.exe .\yuanshen.dict.yaml
.\genshin_pinyin_add.exe .\yuanshen.dict.yaml .\my_rules.json  # 可选的名单路径
```

## 流水线上下文

`Genshin_Words` 词库的完整处理流程：

| 步骤 | 工具                           | 作用                                                 |
| ---- | ------------------------------ | ---------------------------------------------------- |
| 1    | `extract_words.py`             | 按 commit 差集解析 Snap.Metadata 的 CHS JSON 文本，连同手工 `final_npc.txt` 一起切词、去重，产出 `after_del.txt`，再由人工审校 |
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
