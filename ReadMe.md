# 原神词库

这是为rime输入法（小狼毫｜鼠须管）编写的一个**未经特别详细筛选**的原神词库，含有所有怪物、人物（圣遗物、武器、天赋、命座、故事）、食物、家具等的名称。（任务名仅有主线与部分手动添加的名称）

## 更新方式：手动更新

1. 将 [Snap.Metadata](https://github.com/DGP-Studio/Snap.Metadata) 克隆到本地
    - 示例：`git clone https://github.com/DGP-Studio/Snap.Metadata --depth=30`
2. 根据上版本的更新，选择性diff增量内容并输出到指定文件
    - 示例：`git diff c6495f085a main -- "*CHS*" > new.diff`
3. 正则筛选，运行 `regex.py` 文件，打开生成的 `./after.txt`
4. 手动断句
5. 运行 `merge.py`
6. 上传 `yuanshen.dict.yaml`

## 致谢

感谢**以下项目**提供的数据：

- [Yap](https://github.com/Alex-Beng/Yap) - @Alex-Beng
- [BetterGI](https://github.com/babalae/better-genshin-impact/) - @babalae
- [Snap.Metadata](https://github.com/DGP-Studio/Snap.Metadata) - @DGP-Studio
- 我自己的经验
