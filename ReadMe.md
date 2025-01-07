# 原神词库

这是为rime输入法（小狼毫｜鼠须管）编写的一个**未经特别详细筛选**的原神词库，含有所有圣遗物、怪物、人物、食物等的名称。（任务名暂时不包含在内）

## 使用方法

1. 将 [Snap.Metadata](https://github.com/DGP-Studio/Snap.Metadata) 克隆到本地
    - 示例：`git clone https://github.com/DGP-Studio/Snap.Metadata --depth=30`
2. 根据上版本的更新，选择性diff增量内容并输出到指定文件
    - 示例：`git diff c59d8c79d main "*CHS*" |grep "^+" >new.diff`
3. 将 `new.diff` 置于本目录下，运行 `data_wash.py`
4. 将生成的 `out.txt` 手动处理

## How to use

Download all json files from [Genshin/CHS](https://github.com/DGP-Studio/Snap.Metadata/tree/main/Genshin/CHS) and put it into `.\raw\`.

run `data_collect.py`

run `data_wash.py`
