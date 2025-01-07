import regex as re
import io
import pandas as pd
import os


original_diff = io.open(r"./original.diff", "r", encoding="utf8")
fix_init = io.open(r"./out.txt", "w+", encoding="utf8", buffering=1)

regexs_fix = [
    r"^(\+\+\+)(.*)$",
    r"^(\+\s*)([-\d.]+),?$",
    r"^(\+\s*)[\{|\[|\]|\},]{1,3}$",
    r"^(\+\s*\")([a-zA-Z0-9/\\_-]{1,})\":\s([^\u4e00-\u9fff\u3400-\u4dbf\uf900-\ufaff]+)$",
    r".*\{param.*",
    r"^(\+\s{8}\")(Title)\":\s(.*)$",
    r"^(\+\s{4}\")Cv(.*?)\":\s(.*)$",
    r"^(\+\s{4}\")(TypeDescription)\":\s(.*)$",
]

# lines = 10000
chinese = False
for i in original_diff.readlines():
    print(i)
    # lines -= 1
    # if lines == 0:
    #    break

    result = [None]
    for j in regexs_fix:
        temp = re.match(j, i)
        # print(temp, j)
        if temp not in result:
            result.append("temp")
            break
    if result.__len__() == 1:
        print(i)
        fix_init.write(i)
original_diff.close()
fix_init.close()

fix_init = io.open(r"./out.txt", "r", encoding="utf8")
fix_other = io.open(r"./result.txt", "w+", encoding="utf8", buffering=1)
reg_groups = [
    (r"\+\s+\"(.*?)\":\s\"(.*)[\"|].{0,2}", [2]),
    (r"^(.*)·其.$", [1]),
    (r"(.*)?(<i>|<b>)(.*?)(</i>|</b>)(.*?)", [1, 3, 5]),
    (r"\+\s+\"([^:]+)\"$", [1]),
]
for i in fix_init.readlines():
    # print(i)
    # +    "Title": "大地勘探·万火燎灼之原·其二",
    for each in reg_groups:
        temp = re.match(each[0], i)
        string = ""
        if temp != None:
            for every in each[1]:
                string = temp.group(every) + string
            i = string
    i = re.sub(r"<color=#[A-Z0-9]*?>", "", i)
    i = re.sub(r"</color>", "", i)
    i = i.replace(",", "，")
    i = i.replace("\\n", "\n")
    i = i.replace("\n\n", "\n")
    if i.endswith("\n\n"):
        i.removesuffix("\n")
    elif i.endswith("\n"):
        pass
    else:
        i = i + "\n"
    fix_other.write(i)
fix_other.close()


df = pd.read_csv("./result.txt", encoding="utf8", header=None)
df = df.drop_duplicates(ignore_index=True)

df.to_csv("./after.txt", encoding="utf8", header=None, index=None, index_label=None)

os.remove("./out.txt")
os.remove("./result.txt")
