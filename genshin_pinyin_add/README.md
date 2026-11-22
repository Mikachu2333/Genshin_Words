# genshin_pinyin_add

读取 Rime 的 `yuanshen.dict.yaml`，生成 Fcitx5 的 `yuanshen_fcitx5.txt`。普通词条按字音权重与名单规则展开；手工 Rime 拼音作为初始读音，名单可重组对应字位，原权重保留。输出音节以单引号分隔。

## 读音选择

使用 [雾凇拼音](https://github.com/iDvel/rime-ice) 的字音权重表，共 8,183 个字、8,698 个无声调读音。权重基于字频并经人工调整；完整来源、固定提交和比较结果见 [拼音仓库调研](PINYIN_RESEARCH.md)。

默认保留权重达到该字最高权重 **0.1%** 的读音，再取权重最高的两个。修改 [src/main.rs](src/main.rs) 中的 `MIN_READING_FREQUENCY_PERCENT` 后重新构建即可调整阈值，支持 `0.0..=100.0` 范围内的小数百分比，例如 `0.5` 表示 0.5%、`5.0` 表示 5%；`0.0` 表示所有有权重的读音参与筛选，两个读音的上限仍然生效。相同权重按拼音字母顺序决定先后。

| 字 | 来源权重 | 权重筛选结果 |
| --- | --- | --- |
| 长 | chang=3,100,060；zhang=3,100,060 | chang、zhang |
| 还 | hai=6,776,145；huan=406,569 | hai、huan |
| 哪 | na=998,544；ne=1；nei=1 | na |
| 什 | shen=482,836；shi=1,000 | shen、shi |
| 莘 | shen=6,743；xin=1 | shen |

权重表缺失的字沿用 `pinyin` 的首选读音；需要其他读音时，用白名单指定。

## 黑白名单

默认读取输入词典同目录的 [pinyin_rules.json](../pinyin_rules.json)，可直接手动修改；也可以用第二个命令行参数选择配置文件。

优先级为：**词级白名单 > 字级白名单 > 黑名单 > 自动读音**。词级白名单每个字用一个字符串指定读音，如 `"包扎": {"扎": "za"}`。字级白名单单读音用字符串，双读音用固定两项数组，如 `"莘": ["xin", "shen"]`。白名单给出该位置的完整读音集合。

下面是测试配置，完整文件在 [tests/fixtures/pinyin_rules.example.json](tests/fixtures/pinyin_rules.example.json)：

```json
{
  "whitelist": {
    "characters": {
      "莘": ["xin", "shen"]
    },
    "words": {
      "若陀": {"若": "re"},
      "阿房宫赋": {"阿": "e", "房": "pang"}
    }
  },
  "blacklist": {
    "characters": {"着": "zhuo", "阿": "e", "房": "pang"}
  }
}
```

`whitelist.characters` 对所有包含该字的词条生效，包括手工注音词条。`whitelist.words` 对包含该词段的词条生效，并只替换匹配词段中的对应位置；例如 `若陀若` 的第一个“若”采用词级名单，最后一个“若”采用普通规则。匹配会覆盖所有出现位置，也支持重叠词段；较长词段优先，等长且互相矛盾的配置报错。

黑名单只包含 `characters`，每个字用一个字符串指定禁止的读音，例如 `"地": "di"` 禁止派生 `di`。程序先取频率最高的两个候选，再删除被禁止的读音；删除后不补入第三个候选。需要保留其中一个读音时，禁止另一个即可。上述配置会让 `阿房宫赋` 输出 `e'pang'gong'fu`，其他词条中的“阿、房”使用 `a、fang`。

拼音使用小写 ASCII，一个字符串对应一个音节，`ü` 写作 `v`。配置错误或过滤后没有可用读音时，程序报错并保留已有输出文件。

手工词条未命中名单的字位沿用原拼音；应用名单时要求每个字对应一个拼音音节。已有手工读音被黑名单禁止且没有白名单覆盖时，程序报错，便于修正对应词条或规则。

## 运行与验证

```powershell
cargo build --release
Copy-Item -LiteralPath 'target/release/genshin_pinyin_add.exe' -Destination '../genshin_pinyin_add.exe' -Force
.\target\release\genshin_pinyin_add.exe ..\yuanshen.dict.yaml
.\target\release\genshin_pinyin_add.exe ..\yuanshen.dict.yaml ..\my_rules.json

cargo fmt --check
cargo test
cargo clippy --all-targets -- -D warnings
```

上述命令在 `genshin_pinyin_add` 目录执行；复制可执行文件后，`merge.py` 也会使用新构建的阈值设置。

最终结果调用 `sort_lib::sort_fcitx5_lines` 排序，按无声调拼音排列词组、将同词异音集中。Rime 源词典的整行去重由前序 `genshin_pinyin_sort` 执行。

程序保留 16 MiB 输入、16 KiB 单行、256 字/词、1,000,000 组合/词、512 MiB 输出及 1 MiB 名单配置的限制。输出先在限额内缓冲排序，再写入同目录临时文件并原子替换。

## 更新权重数据

固定数据位于 [data/pinyin_reading_weights.tsv](data/pinyin_reading_weights.tsv)，来源及原始文件 SHA-256 写在文件头。数据沿用上游 GPL-3.0，许可证保存在 [data/LICENSE.rime-ice](data/LICENSE.rime-ice)。

取得所需版本的 `cn_dicts/8105.dict.yaml` 后，在本项目目录运行：

```powershell
python tools/import_reading_weights.py path/to/8105.dict.yaml --revision '927e1cf1d3eeb9bf4be85e3f684dba4dd485b6f6'
cargo test
cargo build --release
```

导入器只使用 Python 标准库，生成可离线构建的无声调字音权重表，按字排序、字内按权重降序排列，并检查重复读音和字段格式。更新上游版本时，将 `--revision` 改为对应文件的完整提交号。
