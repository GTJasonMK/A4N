"""Story, blueprint, outline, and chapter body fixtures."""

from __future__ import annotations

from typing import Any

def character(name: str, character_type: str, relation: str) -> dict[str, Any]:
    return {
        "name": name,
        "character_type": character_type,
        "gender": "女" if name == "林岚" else "男",
        "age": "28岁",
        "appearance": "深色风衣，随身携带一只旧怀表",
        "personality": "冷静、敏锐，在压力下仍会坚持查明真相",
        "background": "与旧车站和失踪的实验钟有一段被掩盖的过去",
        "motivation": "阻止钟楼改写城市时间并找出信件来源",
        "arc_summary": "从独自承担秘密到学会信任同伴并接受无法改写的过去",
        "relationship_to_protagonist": relation,
        "arc_stages": {
            "start": "对异常保持戒备",
            "core_wound": "曾因一次错误选择失去重要亲人",
            "desire": "掌控所有可能失控的时间变量",
            "need": "接受合作与有限性",
            "secret": "知道实验钟曾经启动过一次",
            "voice_style": "短句、克制，关键时刻直指事实",
            "turning_point": "在钟楼停摆前选择相信同伴",
            "end": "主动承担守护真实时间线的责任",
        },
    }


def blueprint_response(stage: str) -> dict[str, Any]:
    if stage == "story_core":
        return {
            "title": "逆行时钟",
            "target_audience": "喜欢都市悬疑与轻科幻的成年读者",
            "genre": "都市时间悬疑",
            "style": "克制、具象、以细节推进谜团",
            "tone": "紧张中保留温暖",
            "one_sentence_summary": "一封来自未来的信迫使林岚进入钟楼，阻止城市被七分钟的时间裂缝吞没。",
            "full_synopsis": (
                "暴雨夜，档案修复师林岚在末班车上发现一封写给十年后自己的信，信中预告旧车站会停电，"
                "并要求她在十二点前进入废弃钟楼。她循线找到失踪实验钟的痕迹，也遇到修表师周衡。"
                "两人发现城中所有钟表每天都会慢七分钟，被偷走的时间则汇入钟楼地下装置。幕后维护者顾沉"
                "相信只要累积足够时间，就能回到事故发生前救回家人。林岚逐步确认自己童年的记忆也被装置"
                "改写，而未来来信正是她在另一条失败时间线留下的自救信号。调查中，周衡隐瞒了父亲参与实验"
                "的事实，双方信任一度破裂。随着时间裂缝扩大，车站中的人开始重复同一段动作，城市边缘出现"
                "无法跨越的静止区域。林岚必须在保留一段能让亲人复生的虚假过去，与关闭装置、接受真实失去"
                "之间作出选择。最终她与周衡重新校准主钟，用未来信件中的齿轮暗记逆转能量流。顾沉在最后时刻"
                "看见被困时间中的家人并选择放手。城市恢复正常，但所有人都失去了裂缝期间的记忆，只有林岚"
                "的怀表仍固定慢七分钟，提醒她另一条时间线并未彻底消失。"
            ),
        }
    if stage == "world":
        return {
            "world_setting": {
                "premise": "现代城市地下存在一台能收集人们零碎时间的实验钟。",
                "core_rules": "被收集的时间会让现实钟表变慢；任何回溯都必须由等量记忆支付。",
                "technology_or_magic": {
                    "source": "量子钟与城市旧电网形成的共振",
                    "rules": ["每次只能回溯七分钟", "同一事件不能连续改写两次"],
                    "limits": ["装置只能覆盖旧车站供电范围"],
                    "costs": ["使用者会失去与目标事件等量的个人记忆"],
                },
                "key_locations": [
                    {"name": "旧车站钟楼", "description": "停运车站中央的四面钟楼", "story_function": "主谜团与终局场所"},
                    {"name": "周衡修表铺", "description": "保存实验钟零件和旧档案的小店", "story_function": "调查据点"},
                ],
                "factions": [
                    {"name": "守时人", "description": "秘密维护实验钟的旧研究团队", "agenda": "避免时间裂缝公开", "representative_characters": ["顾沉"]}
                ],
                "history_timeline": [
                    {"period": "十年前", "event": "实验钟首次失控", "impact": "林岚的童年记忆被改写"},
                    {"period": "现在", "event": "未来来信出现", "impact": "时间裂缝再次扩张"},
                ],
                "culture_and_society": {
                    "values": "城市居民相信准时代表可靠",
                    "class_structure": "普通居民与掌握旧实验秘密的技术圈层并存",
                    "daily_life": "地铁、车站广播和手机时钟逐渐出现七分钟偏差",
                },
                "conflict_sources": [
                    {"name": "失去与挽回", "description": "角色都想改写各自的遗憾", "stakes": "真实时间线可能永久崩塌"}
                ],
                "mysteries_and_foreshadowing": [
                    {"mystery": "未来信件由谁寄出", "early_clues": ["齿轮暗记", "慢七分钟的怀表"], "payoff_direction": "来自失败时间线的林岚"}
                ],
                "genre_promises": ["可验证的时间规则", "层层推进的身份谜团", "以选择而非武力完成终局"],
            }
        }
    if stage == "characters":
        return {
            "characters": [
                character("林岚", "protagonist", "本人"),
                character("顾沉", "antagonist", "试图改写时间的对手"),
                character("周衡", "supporting", "调查搭档与潜在恋人"),
            ]
        }
    if stage == "relationships":
        evolution = {
            "start": "互相试探",
            "tension": "双方都隐瞒与实验钟有关的过去",
            "turning_points": ["共同经历车站停电", "坦白父辈参与实验"],
            "hidden_truth": "两人的家人都曾参与第一次实验",
            "story_function": "推动调查并承载信任主题",
            "end": "共同守护真实时间线",
        }
        return {
            "relationships": [
                {"relationship_type": "friend", "character_a": "林岚", "character_b": "周衡", "description": "从互疑到并肩调查", "strength": 7, "evolution": evolution},
                {"relationship_type": "enemy", "character_a": "林岚", "character_b": "顾沉", "description": "对是否应改写过去持相反选择", "strength": 8, "evolution": {**evolution, "end": "理解彼此但选择不同道路"}},
            ]
        }
    raise ValueError(f"unknown blueprint stage: {stage}")


def inspiration_response() -> dict[str, Any]:
    return {
        "ai_message": "素材已经足够完整：旧车站、未来来信、七分钟时间裂缝和必须放下过去的主题都已确定。",
        "next_question": None,
        "progress_summary": "核心设定、类型、文风、主角、冲突、对手、催化事件、主题和篇幅均已确认。",
        "ui_control": {"type": "info_display"},
        "conversation_state": {"round": 1, "chapter_count": 6},
        "is_complete": True,
    }


def part_outline_response(part_number: int) -> dict[str, Any]:
    if part_number == 1:
        return {
            "title": "来信与失去的七分钟",
            "summary": (
                "暴雨夜，林岚在末班车上收到一封来自未来的信，信中准确预告旧车站停电，并要求她在午夜前进入废弃钟楼。"
                "她循着信封上的齿轮暗记找到周衡的修表铺，两人确认全城钟表每天都会慢七分钟。调查从车站档案、异常广播和反复出现的乘客动作展开，"
                "林岚逐步发现自己十年前的记忆存在人为拼接。周衡虽然提供实验钟零件，却隐瞒父亲曾参与第一次事故，双方在地下机房遭遇时间回卷后首次产生裂痕。"
                "分部末尾，未来信件显露只有林岚本人知道的童年暗号，证明寄信者与她共享记忆，也把寻找幕后操作者顾沉和进入钟楼核心层作为下一阶段目标。"
            ),
            "theme": "面对被篡改的记忆，信任必须建立在主动求证之上",
            "key_events": [
                "未来来信预告旧车站停电",
                "林岚与周衡确认七分钟时间偏差",
                "地下机房发生第一次时间回卷",
                "信件出现林岚的童年暗号",
            ],
            "character_focus": {
                "林岚": "从怀疑异常到主动追查自己的缺失记忆",
                "周衡": "以协助者身份出现，同时埋下隐瞒父辈往事的矛盾",
                "顾沉": "通过旧研究记录和监控残影建立威胁",
            },
        }
    return {
        "title": "钟楼终局与真实时间线",
        "summary": (
            "林岚和周衡进入钟楼核心层，发现顾沉一直把全城被偷走的七分钟汇入实验钟，希望回到事故前救回家人。随着裂缝扩大，车站乘客不断重复同一动作，"
            "城市边缘也出现无法跨越的静止区。周衡坦白父亲参与实验的真相，并交出能逆转能量流的校准齿轮，林岚决定暂时相信他。顾沉让她看见一条亲人仍然活着的虚假时间线，"
            "迫使她在个人愿望与整座城市的真实记忆之间选择。林岚最终依照未来信件中的暗记重新校准主钟，主动放弃改写过去；顾沉在看见被困时间中的家人后也选择停手。"
            "城市恢复正常，多数人忘记裂缝经历，林岚与周衡则保留一只固定慢七分钟的怀表，既完成当前冲突，也留下另一条时间线尚未彻底关闭的余韵。"
        ),
        "theme": "真正的修复不是改写失去，而是选择承担真实",
        "key_events": [
            "顾沉揭示收集七分钟的真实目的",
            "周衡坦白父辈参与实验并交出校准齿轮",
            "林岚拒绝虚假时间线并逆转实验钟",
            "城市恢复而怀表继续慢七分钟",
        ],
        "character_focus": {
            "林岚": "完成从控制过去到接受有限性的核心弧光",
            "周衡": "以坦白和行动修复与林岚的信任",
            "顾沉": "从执念对手转为理解代价后主动放手",
        },
    }


def chapter_outline_response(chapter_number: int) -> dict[str, Any]:
    titles = [
        "雨夜来信",
        "慢七分钟的钟",
        "地下机房回卷",
        "周衡的隐瞒",
        "虚假时间线",
        "真实时间归位",
    ]
    title = titles[chapter_number - 1]
    return {
        "title": title,
        "summary": f"第{chapter_number}章围绕“{title}”推进调查，让林岚更接近实验钟与未来来信的真相。",
        "beats": [f"发现第{chapter_number}条时间异常线索", "林岚与周衡据此推进调查"],
        "key_scenes": [f"第{chapter_number}章关键场景：旧车站与钟楼"],
        "pacing_score": 0.72 + chapter_number * 0.03,
        "foreshadowing_setup": ["怀表始终慢七分钟"] if chapter_number < 6 else [],
        "foreshadowing_payoff": ["未来来信中的齿轮暗记得到解释"] if chapter_number == 6 else [],
        "characters_involved": ["林岚", "周衡"] + (["顾沉"] if chapter_number >= 4 else []),
        "word_count_target": 2800 + chapter_number * 100,
    }


def chapter_body_response() -> str:
    return (
        "雨水沿着末班车的车窗斜斜滑下，林岚把那封没有邮戳的信翻到第三遍。信封正面写着她的名字，"
        "背面却标着十年后的日期。十二点前进入旧车站钟楼，否则整座城会失去七分钟——字迹和她一模一样。\n\n"
        "列车驶入废弃站台旁的弯道时，车厢灯忽然熄灭。黑暗里先响起一声机械钟鸣，接着所有人的手机同时亮起，"
        "时间从二十三点四十二分跳回二十三点三十五分。乘客没有察觉，仍重复着熄灯前的动作；只有林岚掌心那只旧怀表，"
        "秒针逆着刻度走了整整一圈。\n\n"
        "她在旧车站下车。雨幕后的钟楼四面表盘停在不同时间，最慢的一面恰好落后七分钟。站厅深处传来金属碰撞声，"
        "修表师周衡提着工具箱从封闭通道里走出。他看见信封角落的齿轮暗记，神色第一次失去从容。\n\n"
        "“这个记号十年前就该被烧掉了。”周衡说。\n\n"
        "林岚没有把信交给他，只展开其中最后一页。纸上新浮出一行被雨水般的墨迹浸透的字：别相信替你校准时间的人。"
        "与此同时，钟楼顶端传来第二声钟鸣。站厅广告屏里的画面倒退七秒，周衡身后的影子却没有跟着移动。"
        "林岚合上怀表，沿着封闭通道向地下走去。她知道信中的第一个预告已经发生，而距离午夜只剩十八分钟。"
    )
