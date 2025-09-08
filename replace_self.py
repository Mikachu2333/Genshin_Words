import re
import regex


def delete_sum_symbol(text: str) -> str:
    # 删除不以加号开头的行
    text = regex.re.sub(r"^(?!\+).*$", "", text, flags=re.MULTILINE)
    # 删除三个加号
    text = regex.re.sub(r"^\+{3}.*$", "", text, flags=re.MULTILINE)
    # 删除加号
    text = regex.re.sub(r"^\+(.*)$", r"\1", text, flags=re.MULTILINE)
    return text


def delete_cv(text: str) -> str:
    # Cv替换
    text = regex.re.sub(
        r"^.*?(CvChinese|CvJapanese|CvEnglish|CvKorean).*?$",
        "",
        text,
        flags=re.MULTILINE,
    )
    return text


def delete_none_chinese(text: str) -> str:
    # 如果一行内容不含中文，删除该行
    text = regex.re.sub(
        r"^[\+A-Za-z0-9_',\"\[\]\{\};:\d\./\\\s-]+$", "", text, flags=re.MULTILINE
    )
    return text


def delete_style(text: str) -> str:
    # 删除{LINK#S11155}.*{/LINK}
    text = regex.re.sub(
        r"\{LINK#.*?\}(.*?)\{/LINK\}", r"『\1』", text, flags=re.MULTILINE
    )

    # 删除<color=#FFD780FF>.*</color>
    text = regex.re.sub(
        r"<color=#[0-9a-fA-F]{4,8}>(.*?)</color>", r"『\1』", text, flags=re.MULTILINE
    )
    text = regex.re.sub(
        r"(.*?)<i>(.*?)</i>(.*?)", r"\1『\2』\3", text, flags=re.MULTILINE
    )

    # 上面两个会导致重复的『，』，替换为一个
    text = regex.re.sub(r"『『", r"『", text, flags=re.MULTILINE)
    text = regex.re.sub(r"』』", r"』", text, flags=re.MULTILINE)
    text = regex.re.sub(r"『『", r"『", text, flags=re.MULTILINE)
    text = regex.re.sub(r"』』", r"』", text, flags=re.MULTILINE)

    # {PARAM#P1192101|3S100}
    text = regex.re.sub(r"\{PARAM#.*?\|.*?\}", r"", text, flags=re.MULTILINE)
    # |{param7:F1}秒
    text = regex.re.sub(
        r"\|\{param.*?[:]{0,1}.*?\}[秒]{0,1}", r"", text, flags=re.MULTILINE
    )
    text = regex.re.sub(r"\{param.*?[:]{0,1}.*?\}", r"", text, flags=re.MULTILINE)
    return text


def delete_fight(text: str) -> str:
    # 伤害具体数值
    text = re.sub(r"^\s+\".*?伤害\|.*", r"", text, flags=re.MULTILINE)
    text = re.sub(r"^\s+\".*?体力消耗\|.*", r"", text, flags=re.MULTILINE)
    text = re.sub(r"^\s+\".*?持续时间\|.*", r"", text, flags=re.MULTILINE)
    return text


def extract_content(text: str) -> str:
    # 提取正文
    text = regex.re.sub(
        r"^\s*\".*?\":\s*\"(.*)\"[,\]\}]*$", r"\1", text, flags=re.MULTILINE
    )
    text = regex.re.sub(r"^\s+\"(.*)\"[,]{0,1}$", r"\1", text, flags=re.MULTILINE)

    return text


def delete_chars(text: str) -> str:
    # 删除角色介绍等
    text = regex.re.sub(r"^下落攻击·.*$", r"", text, flags=re.MULTILINE)
    text = regex.re.sub(r"^元素战技·.*$", r"", text, flags=re.MULTILINE)
    text = regex.re.sub(r"^元素爆发·.*$", r"", text, flags=re.MULTILINE)
    text = regex.re.sub(r"^打开宝箱·.*$", r"", text, flags=re.MULTILINE)
    text = regex.re.sub(r"^生命值低·.*$", r"", text, flags=re.MULTILINE)
    text = regex.re.sub(r"^同伴生命值低·.*$", r"", text, flags=re.MULTILINE)
    text = regex.re.sub(r"^倒下·.*$", r"", text, flags=re.MULTILINE)
    text = regex.re.sub(r"^普通受击·.*$", r"", text, flags=re.MULTILINE)
    text = regex.re.sub(r"^重受击·.*$", r"", text, flags=re.MULTILINE)
    text = regex.re.sub(r"^加入队伍·.*$", r"", text, flags=re.MULTILINE)
    text = regex.re.sub(r"^角色详细$", r"", text, flags=re.MULTILINE)
    text = regex.re.sub(r"^角色故事.*$", r"", text, flags=re.MULTILINE)
    text = regex.re.sub(r"^初次见面.*$", r"", text, flags=re.MULTILINE)
    text = regex.re.sub(r"^闲聊·$", r"", text, flags=re.MULTILINE)
    text = regex.re.sub(r"^下雨的时候.*$", r"", text, flags=re.MULTILINE)
    text = regex.re.sub(r"^打雷的时候.*$", r"", text, flags=re.MULTILINE)
    text = regex.re.sub(r"^下雪的时候.*$", r"", text, flags=re.MULTILINE)
    text = regex.re.sub(r"^刮大风了.*$", r"", text, flags=re.MULTILINE)
    text = regex.re.sub(r"^在沙漠的时候.*$", r"", text, flags=re.MULTILINE)
    text = regex.re.sub(r"^早上好.*$", r"", text, flags=re.MULTILINE)
    text = regex.re.sub(r"^中午好.*$", r"", text, flags=re.MULTILINE)
    text = regex.re.sub(r"^晚上好.*$", r"", text, flags=re.MULTILINE)
    text = regex.re.sub(r"^晚安.*$", r"", text, flags=re.MULTILINE)
    text = regex.re.sub(r"^关于.*?自己·.*$", r"", text, flags=re.MULTILINE)
    text = regex.re.sub(r"^关于我们·.*$", r"", text, flags=re.MULTILINE)
    text = regex.re.sub(r"^有什么想要分享.*$", r"", text, flags=re.MULTILINE)
    text = regex.re.sub(r"^感兴趣的见闻·.*$", r"", text, flags=re.MULTILINE)
    text = regex.re.sub(r"^关于(.*?)…$", r"\1", text, flags=re.MULTILINE)
    text = regex.re.sub(r"^关于我们·.*$", r"", text, flags=re.MULTILINE)
    text = regex.re.sub(r"^想要了解.*?·其.*$", r"", text, flags=re.MULTILINE)
    text = regex.re.sub(r"^.*?的爱好….*$", r"", text, flags=re.MULTILINE)
    text = regex.re.sub(r"^.*?的烦恼….*$", r"", text, flags=re.MULTILINE)
    text = regex.re.sub(r"^喜欢的食物…$", r"", text, flags=re.MULTILINE)
    text = regex.re.sub(r"^讨厌的食物…$", r"", text, flags=re.MULTILINE)
    text = regex.re.sub(r"^收到赠礼·.*$", r"", text, flags=re.MULTILINE)
    text = regex.re.sub(r"^生日…$", r"", text, flags=re.MULTILINE)
    text = regex.re.sub(r"^突破的感受·*$", r"", text, flags=re.MULTILINE)
    return text


def delete_items(text: str) -> str:
    # 购买
    text = regex.re.sub(r"^洞天百宝·摆设图纸购买习得$", r"", text, flags=re.MULTILINE)
    text = regex.re.sub(r"^捕捉获得$", r"", text, flags=re.MULTILINE)
    text = regex.re.sub(r"^.*奇馈宝箱奖励$", r"", text, flags=re.MULTILINE)
    text = regex.re.sub(r"^世界任务获取$", r"", text, flags=re.MULTILINE)
    text = regex.re.sub(r"^限时活动获取$", r"", text, flags=re.MULTILINE)
    text = regex.re.sub(r"^购买纪行·珍珠之歌获取$", r"", text, flags=re.MULTILINE)
    text = regex.re.sub(r"^.*区域特产$", r"", text, flags=re.MULTILINE)
    text = regex.re.sub(r"^素材$", r"", text, flags=re.MULTILINE)
    text = regex.re.sub(r"^锻造用矿石$", r"", text, flags=re.MULTILINE)
    text = regex.re.sub(r"^.*区域特产$", r"", text, flags=re.MULTILINE)
    text = regex.re.sub(r"^角色天赋素材$", r"", text, flags=re.MULTILINE)
    text = regex.re.sub(r"^食物$", r"", text, flags=re.MULTILINE)
    text = regex.re.sub(r"^食谱$", r"", text, flags=re.MULTILINE)
    text = regex.re.sub(r"^食谱：(.*)$", r"\1", text, flags=re.MULTILINE)
    text = regex.re.sub(
        r"^步骤详实的食谱,记载着.*的制作方法。$",
        r"",
        text,
        flags=re.MULTILINE,
    )
    text = regex.re.sub(r"^食材$", r"", text, flags=re.MULTILINE)
    text = regex.re.sub(r"^鱼饵$", r"", text, flags=re.MULTILINE)
    text = regex.re.sub(r"^角色与武器培养素材$", r"", text, flags=re.MULTILINE)
    text = regex.re.sub(r"^角色培养素材$", r"", text, flags=re.MULTILINE)
    text = regex.re.sub(r"^武器突破素材$", r"", text, flags=re.MULTILINE)
    text = regex.re.sub(r"^任务道具$", r"", text, flags=re.MULTILINE)
    text = regex.re.sub(r"^消耗品$", r"", text, flags=re.MULTILINE)
    text = regex.re.sub(r"^「(.*)」的种子$", r"\1", text, flags=re.MULTILINE)
    text = regex.re.sub(
        r"^通过「化种匣」获取的种子，富有活力，品质上佳，种植于.*?后，可在一段时间后生长为「.*?」。$",
        r"",
        text,
        flags=re.MULTILINE,
    )
    text = regex.re.sub(r"^鱼$", r"", text, flags=re.MULTILINE)
    text = regex.re.sub(r"^冒险道具$", r"", text, flags=re.MULTILINE)
    text = regex.re.sub(r"^旋曜玉帛·其.*$", r"", text, flags=re.MULTILINE)
    text = regex.re.sub(r"^鱼竿$", r"", text, flags=re.MULTILINE)
    text = regex.re.sub(r"^小道具$", r"", text, flags=re.MULTILINE)
    text = regex.re.sub(r"^图谱：(.*)$", r"\1", text, flags=re.MULTILINE)
    text = regex.re.sub(
        r"^.*掌握后，能复刻出(.*)。$",
        r"\1",
        text,
        flags=re.MULTILINE,
    )
    text = regex.re.sub(
        r"^步骤详实的图纸，记载着(.*)的制作方法。\n使用后可进入制作界面查看。$",
        r"\1",
        text,
        flags=re.MULTILINE,
    )
    text = regex.re.sub(r"^摆设图纸$", r"", text, flags=re.MULTILINE)
    return text


def replace_text_line(text: str) -> str:
    # 换行
    text = regex.re.sub(r"\\n", r"\n", text, flags=re.MULTILINE)

    return text


def delete_num(text: str) -> str:
    # 数字
    text = regex.re.sub(r"^在(.*)开启\d+个宝箱。$", r"\1", text, flags=re.MULTILINE)
    text = regex.re.sub(
        r"^在(.*)完成\d+个大世界限时挑战。$", r"\1", text, flags=re.MULTILINE
    )
    text = regex.re.sub(
        r"^点亮.*?区域中，(.*)的地图。$",
        r"\1",
        text,
        flags=re.MULTILINE,
    )
    text = regex.re.sub(
        r"^解锁.*?区域中，(.*)所有传送锚点。$",
        r"\1",
        text,
        flags=re.MULTILINE,
    )
    text = regex.re.sub(
        r"^解除.*?区域中，(.*)所有.*的封印。$",
        r"\1",
        text,
        flags=re.MULTILINE,
    )
    text = regex.re.sub(
        r"^将.*?的.*?神像供奉至满级。$",
        r"",
        text,
        flags=re.MULTILINE,
    )
    text = regex.re.sub(
        r"^将(.*?)共建至满级。$",
        r"\1",
        text,
        flags=re.MULTILINE,
    )
    text = regex.re.sub(
        r"^与其他玩家一同战胜(.*?)。$",
        r"\1",
        text,
        flags=re.MULTILINE,
    )
    text = regex.re.sub(
        r"·其之.{1,2}$",
        r"",
        text,
        flags=re.MULTILINE,
    )
    text = regex.re.sub(
        r"·其.{1,2}$",
        r"",
        text,
        flags=re.MULTILINE,
    )
    text = regex.re.sub(
        r"·第[.]{1,2}辑$",
        r"",
        text,
        flags=re.MULTILINE,
    )
    text = regex.re.sub(
        r"？？？",
        r"",
        text,
        flags=re.MULTILINE,
    )
    text = regex.re.sub(
        r"\?\?\?",
        r"",
        text,
        flags=re.MULTILINE,
    )
    text = regex.re.sub(
        r"·[上中下]$",
        r"",
        text,
        flags=re.MULTILINE,
    )
    text = regex.re.sub(
        r"·卷.*$",
        r"",
        text,
        flags=re.MULTILINE,
    )
    text = regex.re.sub(
        r".*使用后可进入.*查看.*$",
        r"",
        text,
        flags=re.MULTILINE,
    )
    text = regex.re.sub(
        r".*步骤详实的.*?，记载着「(.*?)」的.*$",
        r"\1",
        text,
        flags=re.MULTILINE,
    )
    return text


def replace_multilines(text: str, n: int) -> str:
    # 空行替换
    for _ in range(n):
        text = regex.re.sub(r"\n\n", r"\n", text, flags=re.MULTILINE)
    return text


def replace_pair(symbol: str, text: str) -> str:
    temp = list(symbol)
    if temp.__len__() != 2:
        return text
    regex_str = r"(.*){}(.*?){}(.*)".format(temp[0], temp[1])
    text = regex.re.sub(regex_str, r"\1\n\2\n\3", text, flags=re.MULTILINE)
    return text


def replace_to_newline(text: str) -> str:
    text = replace_pair("「」", text)
    text = replace_pair("「」", text)
    text = replace_pair("『』", text)
    text = replace_pair("『』", text)
    text = replace_pair("『』", text)
    text = replace_pair("「」", text)
    text = replace_pair("「」", text)
    text = replace_pair("「」", text)
    text = replace_pair("『』", text)
    text = replace_pair("『』", text)
    text = replace_pair("（）", text)
    text = replace_pair("《》", text)
    text = replace_pair("()", text)
    text = replace_pair("（）", text)
    text = replace_pair("《》", text)
    text = replace_pair("()", text)
    text = regex.re.sub(r"[·—\-。：？，；、！…]", r"\n", text, flags=re.MULTILINE)

    return text


def replace_percent(text: str) -> str:
    text = regex.re.sub(r"伤害(额外)?提升", r"", text, flags=re.MULTILINE)
    text = regex.re.sub(r"^[\d\.]+[\%点秒]{0,1}$", r"", text, flags=re.MULTILINE)
    text = regex.re.sub(r"^[\d\.]+%/[\d\.]+%$", r"", text, flags=re.MULTILINE)
    text = regex.re.sub(r"^点$", r"", text, flags=re.MULTILINE)
    text = regex.re.sub(r"^持续[\d\.]+秒$", r"", text, flags=re.MULTILINE)
    text = regex.re.sub(
        r"^(暴击伤害|攻击力|暴击率)提高$", r"", text, flags=re.MULTILINE
    )
    text = regex.re.sub(r"^并为装备者恢复$", r"", text, flags=re.MULTILINE)
    text = regex.re.sub(r"^点元素能量$", r"", text, flags=re.MULTILINE)
    text = regex.re.sub(r"^每[\d\.]+秒至多.*$", r"", text, flags=re.MULTILINE)
    text = regex.re.sub(r"^施放.*?后的[\d\.]+秒内.*$", r"", text, flags=re.MULTILINE)
    text = regex.re.sub(r"^.*队伍后台.*$", r"", text, flags=re.MULTILINE)
    text = regex.re.sub(r"^.*攻击力提升.*$", r"", text, flags=re.MULTILINE)
    text = regex.re.sub(r"^.*角色周围.*$", r"", text, flags=re.MULTILINE)
    text = regex.re.sub(r"^.*反应造成的.*$", r"", text, flags=re.MULTILINE)
    text = regex.re.sub(r"^.*上述效果.*$", r"", text, flags=re.MULTILINE)
    text = regex.re.sub(r"^.*触发燃烧.*$", r"", text, flags=re.MULTILINE)
    text = regex.re.sub(r"^.*元素精通.*$", r"", text, flags=re.MULTILINE)
    text = regex.re.sub(r"^.*触发元素反应.*$", r"", text, flags=re.MULTILINE)
    text = regex.re.sub(r"^.*该效果.*$", r"", text, flags=re.MULTILINE)
    text = regex.re.sub(r"^.*造成的伤害.*$", r"", text, flags=re.MULTILINE)
    text = regex.re.sub(r"^.*元素能量上限.*$", r"", text, flags=re.MULTILINE)
    text = regex.re.sub(r"^.*上限提高.*$", r"", text, flags=re.MULTILINE)
    text = regex.re.sub(r"^.*无法叠加.*$", r"", text, flags=re.MULTILINE)
    text = regex.re.sub(r"^.*此效果.*$", r"", text, flags=re.MULTILINE)
    text = regex.re.sub(r"^.*角色触发.*$", r"", text, flags=re.MULTILINE)
    text = regex.re.sub(r"^.*元素伤害.*$", r"", text, flags=re.MULTILINE)
    text = regex.re.sub(r"^.*队伍中的.*$", r"", text, flags=re.MULTILINE)
    text = regex.re.sub(r"^.*充能效率.*$", r"", text, flags=re.MULTILINE)
    text = regex.re.sub(r"^.*将在敌人.*$", r"", text, flags=re.MULTILINE)
    text = regex.re.sub(r"^效果$", r"", text, flags=re.MULTILINE)
    text = regex.re.sub(r"^.{1}$", r"", text, flags=re.MULTILINE)
    text = regex.re.sub(r"^.*装备者.*$", r"", text, flags=re.MULTILINE)
    text = regex.re.sub(r"^.*命之座激活.*$", r"", text, flags=re.MULTILINE)
    text = regex.re.sub(r"^.*的命星$", r"", text, flags=re.MULTILINE)
    text = regex.re.sub(r"^.*激活素材.*$", r"", text, flags=re.MULTILINE)
    text = regex.re.sub(r"^.*暴击.*$", r"", text, flags=re.MULTILINE)
    text = regex.re.sub(r"^.*暴击伤害.*$", r"", text, flags=re.MULTILINE)
    text = regex.re.sub(r"^.*消耗一层.*$", r"", text, flags=re.MULTILINE)
    text = regex.re.sub(r"^.*将在敌人.*$", r"", text, flags=re.MULTILINE)
    text = regex.re.sub(r"^.*将在敌人.*$", r"", text, flags=re.MULTILINE)

    return text
