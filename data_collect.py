# coding=utf-8
import jieba as jb
import io
import paddle

original = io.open("./after.txt", "r", encoding="utf8")
new = io.open("./new.txt", "w+", encoding="utf8", buffering=1)

paddle.enable_static()
jb.enable_paddle()
jb.enable_parallel()
jb.default_encoding("utf8")

test = 1000
for i in original.readlines():
    test -= 1
    if test == 0:
        break
    for j in jb.lcut(i):
        new.write(j + "\n")
