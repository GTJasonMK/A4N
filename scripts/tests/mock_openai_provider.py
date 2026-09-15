#!/usr/bin/env python3
"""Deterministic loopback-only OpenAI-compatible provider for CLI E2E tests."""

from __future__ import annotations

import argparse
import base64
import hashlib
import json
import re
import threading
import time
from datetime import datetime, timedelta, timezone
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from typing import Any


PNG_1X1_BASE64 = (
    "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNk+A8AAQUBAScY42YAAAAASUVORK5CYII="
)
MP4_1S_BASE64 = (
    "AAAAIGZ0eXBpc29tAAACAGlzb21pc28yYXZjMW1wNDEAAAOHbW9vdgAAAGxtdmhkAAAAAAAAAAAAAAAAAAAD6AAAA+gAAQAAAQAAAAAAAAAAAAAAAAEA"
    "AAAAAAAAAAAAAAAAAAABAAAAAAAAAAAAAAAAAABAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAgAAArF0cmFrAAAAXHRraGQAAAADAAAAAAAA"
    "AAAAAAABAAAAAAAAA+gAAAAAAAAAAAAAAAAAAAAAAAEAAAAAAAAAAAAAAAAAAAABAAAAAAAAAAAAAAAAAABAAAAAABAAAAAQAAAAAAAkZWR0cwAAABxl"
    "bHN0AAAAAAAAAAEAAAPoAAAAAAABAAAAAAIpbWRpYQAAACBtZGhkAAAAAAAAAAAAAAAAAAAyAAAAMgBVxAAAAAAALWhkbHIAAAAAAAAAAHZpZGUAAAAA"
    "AAAAAAAAAABWaWRlb0hhbmRsZXIAAAAB1G1pbmYAAAAUdm1oZAAAAAEAAAAAAAAAAAAAACRkaW5mAAAAHGRyZWYAAAAAAAAAAQAAAAx1cmwgAAAAAQAA"
    "AZRzdGJsAAAAuHN0c2QAAAAAAAAAAQAAAKhhdmMxAAAAAAAAAAEAAAAAAAAAAAAAAAAAAAAAABAAEABIAAAASAAAAAAAAAABFUxhdmM2Mi4yOC4xMDIg"
    "bGlieDI2NAAAAAAAAAAAAAAAGP//AAAALmF2Y0MBQsAK/+EAFmdCwArZHsBEAAADAAQAAAMAyDxImSABAAVoy4PLIAAAABBwYXNwAAAAAQAAAAEAAAAU"
    "YnRydAAAAAAAABsoAAAAAAAAABhzdHRzAAAAAAAAAAEAAAAZAAACAAAAABRzdHNzAAAAAAAAAAEAAAABAAAAHHN0c2MAAAAAAAAAAQAAAAEAAAAZAAAA"
    "AQAAAHhzdHN6AAAAAAAAAAAAAAAZAAACjAAAAAkAAAAKAAAACQAAAAkAAAAJAAAACQAAAAkAAAAJAAAACQAAAAkAAAAJAAAACQAAAAkAAAAJAAAACQAA"
    "AAkAAAAJAAAACQAAAAkAAAAJAAAACQAAAAkAAAAJAAAACQAAABRzdGNvAAAAAAAAAAEAAAO3AAAAYnVkdGEAAABabWV0YQAAAAAAAAAhaGRscgAAAAAA"
    "AAAAbWRpcmFwcGwAAAAAAAAAAAAAAAAtaWxzdAAAACWpdG9vAAAAHWRhdGEAAAABAAAAAExhdmY2Mi4xMi4xMDIAAAAIZnJlZQAAA21tZGF0AAACcQYF"
    "//9t3EXpvebZSLeWLNgg2SPu73gyNjQgLSBjb3JlIDE2NSByMzIyMiBiMzU2MDVhIC0gSC4yNjQvTVBFRy00IEFWQyBjb2RlYyAtIENvcHlsZWZ0IDIw"
    "MDMtMjAyNSAtIGh0dHA6Ly93d3cudmlkZW9sYW4ub3JnL3gyNjQuaHRtbCAtIG9wdGlvbnM6IGNhYmFjPTAgcmVmPTMgZGVibG9jaz0xOjA6MCBhbmFs"
    "eXNlPTB4MToweDExMSBtZT1oZXggc3VibWU9NyBwc3k9MSBwc3lfcmQ9MS4wMDowLjAwIG1peGVkX3JlZj0xIG1lX3JhbmdlPTE2IGNocm9tYV9tZT0x"
    "IHRyZWxsaXM9MSA4eDhkY3Q9MCBjcW09MCBkZWFkem9uZT0yMSwxMSBmYXN0X3Bza2lwPTEgY2hyb21hX3FwX29mZnNldD0tMiB0aHJlYWRzPTEgbG9v"
    "a2FoZWFkX3RocmVhZHM9MSBzbGljZWRfdGhyZWFkcz0wIG5yPTAgZGVjaW1hdGU9MSBpbnRlcmxhY2VkPTAgYmx1cmF5X2NvbXBhdD0wIGNvbnN0cmFp"
    "bmVkX2ludHJhPTAgYmZyYW1lcz0wIHdlaWdodHA9MCBrZXlpbnQ9MjUwIGtleWludF9taW49MjUgc2NlbmVjdXQ9NDAgaW50cmFfcmVmcmVzaD0wIHJj"
    "X2xvb2thaGVhZD00MCByYz1jcmYgbWJ0cmVlPTEgY3JmPTIzLjAgcWNvbXA9MC42MCBxcG1pbj0wIHFwbWF4PTY5IHFwc3RlcD00IGlwX3JhdGlvPTEu"
    "NDAgYXE9MToxLjAwAIAAAAATZYiEDPEYoAAm2xwABAsjgACCngAAAAVBmjgZ6gAAAAZBmlQGeoAAAAAFQZpgM9QAAAAFQZqAM9QAAAAFQZqgM9QAAAAF"
    "QZrAM9QAAAAFQZrgM9QAAAAFQZsAM9QAAAAFQZsgM9QAAAAFQZtAM9QAAAAFQZtgM9QAAAAFQZuAM9QAAAAFQZugM9QAAAAFQZvAM9QAAAAFQZvgM9QA"
    "AAAFQZoAM9QAAAAFQZogM9QAAAAFQZpAM9QAAAAFQZpgM9QAAAAFQZqAM9QAAAAFQZqgM9QAAAAFQZrAL9QAAAAFQZrgL9QAAAAFQZsAK9Q="
)


class UnhandledPromptError(ValueError):
    pass


class VideoIdempotencyConflict(ValueError):
    pass


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


def source_span_pairs(combined: str) -> list[tuple[str, str]]:
    return list(
        dict.fromkeys(
            re.findall(
                r'"id"\s*:\s*"(ch\d+:s\d+:f\d+)"\s*,\s*'
                r'"paragraph_index"\s*:\s*\d+\s*,\s*"text"\s*:\s*"([^"]+)"',
                combined,
            )
        )
    )


def source_span_id(combined: str, phrase: str, fallback_index: int = 0) -> str:
    pairs = source_span_pairs(combined)
    for span_id, text in pairs:
        if phrase in text:
            return span_id
    if not pairs:
        raise ValueError("chapter analysis prompt contains no source spans")
    return pairs[min(fallback_index, len(pairs) - 1)][0]


def text_anchor_id(combined: str, phrase: str) -> int:
    for value in embedded_json_objects(combined):
        anchor_id = value.get("id")
        quote = value.get("quote")
        if isinstance(anchor_id, int) and isinstance(quote, str) and phrase in quote:
            return anchor_id
    raise ValueError(f"prompt contains no text anchor for {phrase}")


def named_profile_id(combined: str, name: str, field: str) -> int:
    for value in embedded_json_objects(combined):
        if value.get("name") != name:
            continue
        profile_id = value.get(field)
        if isinstance(profile_id, int) and not isinstance(profile_id, bool):
            return profile_id
    raise ValueError(f"prompt contains no {field} for {name}")


def character_sync_profile_name(combined: str) -> str:
    names = re.findall(r'"profile_name"\s*:\s*"([^"]+)"', combined)
    if not names:
        raise ValueError("character sync prompt contains no profile_name")
    return names[-1]


def object_discovery_response(combined: str) -> dict[str, Any]:
    return {
        "objects": [
            {
                "name": "旧怀表",
                "aliases": ["怀表"],
                "object_type": "token",
                "importance": "primary",
                "visual_description": "林岚掌心随身携带、指针持续逆行的旧式怀表",
                "material_and_state": "旧式表壳，指针处于逆行状态",
                "story_usage": "证明时间异常真实发生的关键线索",
                "visual_usage": "固定林岚手部动作与时间异常的连续性锚点",
                "generation_usage": "保持旧式怀表外观，并明确表现指针逆行",
                "location_context": "林岚掌心",
                "evidence_anchor_ids": [text_anchor_id(combined, "秒针逆着刻度走了整整一圈")],
            },
            {
                "name": "齿轮暗记",
                "aliases": ["齿轮记号"],
                "object_type": "token",
                "importance": "secondary",
                "visual_description": "印在未来来信信封角落、形似齿轮的暗记",
                "material_and_state": "墨迹位于信封角落，来源与年代尚未解释",
                "story_usage": "触发周衡失态并连接十年前的秘密",
                "visual_usage": "把未来来信与周衡的反应固定在同一视觉线索上",
                "generation_usage": "暗记必须保留在信封角落，不得转移到墙面",
                "location_context": "林岚持有的未来来信信封角落",
                "evidence_anchor_ids": [text_anchor_id(combined, "他看见信封角落的齿轮暗记")],
            },
        ]
    }


def character_discovery_response(combined: str) -> dict[str, Any]:
    return {
        "characters": [
            {
                "name": "林岚",
                "aliases": [],
                "importance": "primary",
                "role": "主角",
                "explicit_attributes": {"位置": "旧车站封闭通道", "持有物": "旧怀表与未来来信"},
                "implicit_attributes": {"目标": "调查时间异常", "态度": "警觉"},
                "social_attributes": {"周衡": "暂不信任"},
                "evidence_anchor_id": text_anchor_id(combined, "林岚没有把信交给他"),
            },
            {
                "name": "周衡",
                "aliases": [],
                "importance": "secondary",
                "role": "调查对象",
                "explicit_attributes": {"位置": "旧车站封闭通道", "异常": "影子未随画面回退"},
                "implicit_attributes": {"动机": "尚未明确"},
                "social_attributes": {"林岚": "受到怀疑"},
                "evidence_anchor_id": text_anchor_id(combined, "修表师周衡提着工具箱从封闭通道里走出"),
            },
        ]
    }


def scene_profile_candidates_response(combined: str) -> dict[str, Any]:
    return {
        "scenes": [
            {
                "scene_ref": "station_concourse",
                "scene_profile_id": None,
                "name": "旧车站站厅",
                "aliases": ["站厅"],
                "scene_type": "room",
                "importance": "primary",
                "parent_scene_id": None,
                "base_description": "废弃旧车站中连接站台、钟楼与封闭通道的站厅",
                "visual_description": "停电后的广告屏冷光照出周衡和林岚手中的信封",
                "spatial_layout": "站台入口通向站厅深处，封闭通道入口位于一侧",
                "atmosphere": "停电后的昏暗冷光与机械钟鸣造成紧张感",
                "entry_points": ["废弃站台", "封闭通道"],
                "rules": ["电子画面会发生七秒回退"],
                "evidence_anchor_ids": [text_anchor_id(combined, "站厅深处传来金属碰撞声")],
            },
            {
                "scene_ref": "station_passage",
                "scene_profile_id": None,
                "name": "旧车站封闭通道",
                "aliases": ["封闭通道"],
                "scene_type": "route",
                "importance": "primary",
                "parent_scene_id": None,
                "base_description": "从旧车站站厅向地下延伸的封闭通道",
                "visual_description": "通道从站厅冷光中向地下延伸，尽头仍未揭示",
                "spatial_layout": "入口连接站厅，路径持续向地下延伸",
                "atmosphere": "未知、压迫，时间异常来源仍未显现",
                "entry_points": ["站厅入口", "地下方向"],
                "rules": [],
                "evidence_anchor_ids": [text_anchor_id(combined, "沿着封闭通道向地下走去")],
            },
        ]
    }


def scene_occurrence_response(combined: str) -> dict[str, Any]:
    lin_lan = named_profile_id(combined, "林岚", "profile_id")
    zhou_heng = named_profile_id(combined, "周衡", "profile_id")
    watch = named_profile_id(combined, "旧怀表", "object_profile_id")
    gear_mark = named_profile_id(combined, "齿轮暗记", "object_profile_id")
    return {
        "occurrences": [
            {
                "occurrence_ref": "occ_station_concourse_1",
                "scene_ref": "station_concourse",
                "timeline_position": "暴雨夜午夜前",
                "summary": "周衡看见信封角落的齿轮暗记后失态，林岚随后观察到他的影子没有随画面回退。",
                "mood": "警觉紧张",
                "weather_or_lighting": "停电后的广告屏冷光",
                "source_anchor_ids": [
                    text_anchor_id(combined, "他看见信封角落的齿轮暗记"),
                    text_anchor_id(combined, "周衡身后的影子却没有跟着移动"),
                ],
                "characters": [
                    {"profile_id": lin_lan, "role": "保留未来来信并判断异常"},
                    {"profile_id": zhou_heng, "role": "认出暗记且影子表现异常"},
                ],
                "objects": [
                    {"object_profile_id": watch, "role": "由林岚随身携带"},
                    {"object_profile_id": gear_mark, "role": "位于未来来信信封角落"},
                ],
                "key_events": ["林岚确认周衡身上存在异常"],
                "entry_state": {},
                "exit_state": {},
            },
            {
                "occurrence_ref": "occ_station_passage_1",
                "scene_ref": "station_passage",
                "timeline_position": "第二声钟鸣后",
                "summary": "林岚拒绝交出未来来信，合上怀表并沿封闭通道向地下继续调查。",
                "mood": "坚定而压迫",
                "weather_or_lighting": None,
                "source_anchor_ids": [
                    text_anchor_id(combined, "林岚没有把信交给他"),
                    text_anchor_id(combined, "沿着封闭通道向地下走去"),
                ],
                "characters": [{"profile_id": lin_lan, "role": "进入机房继续调查"}],
                "objects": [
                    {"object_profile_id": watch, "role": "继续作为时间异常线索"},
                    {"object_profile_id": gear_mark, "role": "随林岚持有的信封移动"},
                ],
                "key_events": ["林岚沿封闭通道向地下继续调查"],
                "entry_state": {},
                "exit_state": {},
            },
        ]
    }


def scene_transition_response(combined: str) -> dict[str, Any]:
    lin_lan = named_profile_id(combined, "林岚", "profile_id")
    watch = named_profile_id(combined, "旧怀表", "object_profile_id")
    return {
        "transitions": [
            {
                "from_scene_ref": None,
                "to_scene_ref": "station_concourse",
                "occurrence_ref": "occ_station_concourse_1",
                "movement_type": "enter",
                "route_description": "林岚从废弃站台进入旧车站站厅",
                "time_cost": None,
                "distance_hint": None,
                "participants": [lin_lan],
                "object_refs": [watch],
                "source_anchor_ids": [text_anchor_id(combined, "她在旧车站下车")],
            },
            {
                "from_scene_ref": "station_concourse",
                "to_scene_ref": "station_passage",
                "occurrence_ref": "occ_station_passage_1",
                "movement_type": "travel",
                "route_description": "林岚离开站厅，沿封闭通道向地下走去",
                "time_cost": None,
                "distance_hint": "向地下延伸",
                "participants": [lin_lan],
                "object_refs": [watch],
                "source_anchor_ids": [text_anchor_id(combined, "沿着封闭通道向地下走去")],
            },
        ]
    }


def scene_entity_state_response(combined: str) -> dict[str, Any]:
    lin_lan = named_profile_id(combined, "林岚", "profile_id")
    zhou_heng = named_profile_id(combined, "周衡", "profile_id")
    watch = named_profile_id(combined, "旧怀表", "object_profile_id")
    gear_mark = named_profile_id(combined, "齿轮暗记", "object_profile_id")
    return {
        "entity_states": [
            {
                "scene_ref": "station_concourse",
                "occurrence_ref": "occ_station_concourse_1",
                "entity_type": "character",
                "entity_id": lin_lan,
                "state_key": "possession",
                "state_before": {},
                "state_after": {"object_profile_id": watch},
                "position_description": "旧车站站厅",
                "action_description": "林岚随身持有逆行的旧怀表",
                "source_anchor_ids": [text_anchor_id(combined, "秒针逆着刻度走了整整一圈")],
            },
            {
                "scene_ref": "station_concourse",
                "occurrence_ref": "occ_station_concourse_1",
                "entity_type": "object",
                "entity_id": gear_mark,
                "state_key": "position",
                "state_before": {},
                "state_after": {"location": "未来来信信封角落"},
                "position_description": "林岚持有的信封角落",
                "action_description": "周衡看见信封上的齿轮暗记后失去从容",
                "source_anchor_ids": [text_anchor_id(combined, "他看见信封角落的齿轮暗记")],
            },
            {
                "scene_ref": "station_concourse",
                "occurrence_ref": "occ_station_concourse_1",
                "entity_type": "character",
                "entity_id": zhou_heng,
                "state_key": "condition",
                "state_before": {},
                "state_after": {"shadow": "未随广告画面回退"},
                "position_description": "封闭通道附近",
                "action_description": "周衡的影子停在原地",
                "source_anchor_ids": [text_anchor_id(combined, "周衡身后的影子却没有跟着移动")],
            },
            {
                "scene_ref": "station_passage",
                "occurrence_ref": "occ_station_passage_1",
                "entity_type": "character",
                "entity_id": lin_lan,
                "state_key": "position",
                "state_before": {"scene_ref": "station_concourse"},
                "state_after": {"scene_ref": "station_passage"},
                "position_description": "向地下延伸的封闭通道内",
                "action_description": "林岚沿封闭通道向地下继续调查",
                "source_anchor_ids": [text_anchor_id(combined, "沿着封闭通道向地下走去")],
            },
        ]
    }


def character_sync_evidence_events_response(combined: str) -> dict[str, Any]:
    name = character_sync_profile_name(combined)
    if name == "林岚":
        return {
            "events": [
                {
                    "chapter_number": 1,
                    "event_type": "discovery",
                    "summary": "旧怀表的秒针逆行一圈，林岚确认七分钟时间异常真实发生。",
                    "evidence_anchor_id": text_anchor_id(combined, "秒针逆着刻度走了整整一圈"),
                    "related_characters": [],
                },
                {
                    "chapter_number": 1,
                    "event_type": "decision",
                    "summary": "林岚拒绝交出未来来信，并沿封闭通道向地下继续调查。",
                    "evidence_anchor_id": text_anchor_id(combined, "林岚没有把信交给他"),
                    "related_characters": ["周衡"],
                },
            ]
        }
    if name == "周衡":
        return {
            "events": [
                {
                    "chapter_number": 1,
                    "event_type": "anomaly",
                    "summary": "广告画面回退时，周衡的影子仍停在原地。",
                    "evidence_anchor_id": text_anchor_id(combined, "周衡身后的影子却没有跟着移动"),
                    "related_characters": ["林岚"],
                }
            ]
        }
    return {"events": []}


def character_sync_attribute_changes_response(combined: str) -> dict[str, Any]:
    name = character_sync_profile_name(combined)
    if name == "林岚":
        return {
            "attribute_changes": [
                {
                    "chapter_number": 1,
                    "category": "explicit",
                    "key": "current_location",
                    "operation": "add",
                    "new_value": "旧车站封闭通道",
                    "change_description": "林岚已离开站厅，沿封闭通道向地下移动。",
                    "event_cause": "主动继续调查时间异常",
                    "evidence_anchor_id": text_anchor_id(combined, "沿着封闭通道向地下走去"),
                },
                {
                    "chapter_number": 1,
                    "category": "explicit",
                    "key": "current_goal",
                    "operation": "add",
                    "new_value": "调查时间异常并保持对周衡的警惕",
                    "change_description": "本章明确形成新的即时调查目标。",
                    "event_cause": "周衡的影子表现出异常",
                    "evidence_anchor_id": text_anchor_id(combined, "林岚没有把信交给他"),
                },
            ]
        }
    if name == "周衡":
        return {
            "attribute_changes": [
                {
                    "chapter_number": 1,
                    "category": "explicit",
                    "key": "current_location",
                    "operation": "add",
                    "new_value": "旧车站封闭通道",
                    "change_description": "周衡出现在旧车站封闭通道附近。",
                    "event_cause": "林岚在通道入口观察到他",
                    "evidence_anchor_id": text_anchor_id(combined, "修表师周衡提着工具箱从封闭通道里走出"),
                },
                {
                    "chapter_number": 1,
                    "category": "explicit",
                    "key": "time_anomaly",
                    "operation": "add",
                    "new_value": "影子未随广告画面回退",
                    "change_description": "周衡出现可观察的时间异常特征。",
                    "event_cause": "第二声钟鸣后广告画面倒退",
                    "evidence_anchor_id": text_anchor_id(combined, "周衡身后的影子却没有跟着移动"),
                },
            ]
        }
    return {"attribute_changes": []}


def character_sync_chapter_states_response(combined: str) -> dict[str, Any]:
    name = character_sync_profile_name(combined)
    if name == "林岚":
        state = {
            "chapter_number": 1,
            "state_summary": "确认时间异常后沿封闭通道向地下继续调查，并保持对周衡的警惕。",
            "current_location": "旧车站封闭通道",
            "current_goal": "调查时间异常并检验周衡",
            "emotional_state": "警觉而坚定",
        }
    elif name == "周衡":
        state = {
            "chapter_number": 1,
            "state_summary": "停留在旧车站封闭通道，其异常影子使身份与动机受到怀疑。",
            "current_location": "旧车站封闭通道",
            "current_goal": None,
            "emotional_state": "克制而紧张",
        }
    else:
        state = {
            "chapter_number": 1,
            "state_summary": f"{name} 在本章的状态由当前正文继续承接。",
            "current_location": "旧车站封闭通道",
            "current_goal": None,
            "emotional_state": None,
        }
    return {"chapter_states": [state]}


def character_sync_behaviors_response(combined: str) -> dict[str, Any]:
    name = character_sync_profile_name(combined)
    if name == "林岚":
        return {
            "behaviors": [
                {
                    "chapter_number": 1,
                    "behavior_type": "investigation",
                    "behavior_description": "拒绝交出未来来信后沿封闭通道向地下继续调查。",
                    "original_text_anchor_id": text_anchor_id(combined, "沿着封闭通道向地下走去"),
                    "behavior_tags": ["主动调查", "风险判断"],
                    "related_characters": ["周衡"],
                    "impact_analysis": "推动调查进入时间异常的潜在源头。",
                }
            ]
        }
    if name == "周衡":
        return {
            "behaviors": [
                {
                    "chapter_number": 1,
                    "behavior_type": "anomaly_exposure",
                    "behavior_description": "广告画面回退时影子仍停在原地。",
                    "original_text_anchor_id": text_anchor_id(combined, "周衡身后的影子却没有跟着移动"),
                    "behavior_tags": ["时间异常", "身份疑点"],
                    "related_characters": ["林岚"],
                    "impact_analysis": "加深林岚对周衡身份和动机的怀疑。",
                }
            ]
        }
    return {"behaviors": []}


def implicit_attribute_classification_response(combined: str) -> dict[str, Any]:
    current_attributes = next(
        (
            value["current_implicit_attributes"]
            for value in embedded_json_objects(combined)
            if isinstance(value.get("current_implicit_attributes"), dict)
        ),
        {},
    )
    classifications = {
        key: "conform"
        for key in list(current_attributes)[:1]
        if isinstance(key, str) and key.strip()
    }
    return {
        "classifications": classifications,
        "reasoning": "只对当前上下文中已存在的隐性属性进行分类；没有可用属性时不新增分类。",
        "suggested_new_attributes": [],
    }


def chapter_analysis_facts_response(combined: str) -> dict[str, Any]:
    lin_lan = source_span_id(combined, "林岚把那封没有邮戳的信翻到第三遍")
    old_station = source_span_id(combined, "她在旧车站下车")
    watch = source_span_id(combined, "秒针逆着刻度走了整整一圈")
    zhou_heng = source_span_id(combined, "周衡身后的影子却没有跟着移动")
    gear_mark = source_span_id(combined, "他看见信封角落的齿轮暗记")
    decision = source_span_id(combined, "沿着封闭通道向地下走去")
    return {
        "characters": [
            {"name": "林岚", "source_span_ids": [lin_lan]},
            {"name": "周衡", "source_span_ids": [zhou_heng]},
        ],
        "locations": [
            {"name": "旧车站", "source_span_ids": [old_station]},
            {"name": "封闭通道", "source_span_ids": [decision]},
        ],
        "items": [
            {"name": "旧怀表", "source_span_ids": [watch]},
            {"name": "齿轮暗记", "source_span_ids": [gear_mark]},
        ],
        "event_candidates": [
            {
                "event_ref": "event-clock-reversal",
                "type": "discovery",
                "description": "林岚发现旧车站的电子钟倒退七分钟，而旧怀表仍在逆行。",
                "importance": "high",
                "involved_characters": ["林岚"],
                "source_span_ids": [old_station, watch],
            },
            {
                "event_ref": "event-enter-closed-passage",
                "type": "decision",
                "description": "林岚拒绝交出未来来信，并沿封闭通道向地下继续调查。",
                "importance": "high",
                "involved_characters": ["林岚", "周衡"],
                "source_span_ids": [gear_mark, decision],
            },
        ],
        "foreshadowing_candidates": [
            {
                "foreshadowing_ref": "foreshadow-static-shadow",
                "description": "周衡的影子没有随画面回退，暗示他与时间异常存在更深联系。",
                "category": "character_secret",
                "priority": "high",
                "related_entities": ["周衡", "实验钟"],
                "original_text_span_id": zhou_heng,
            }
        ],
    }


def chapter_analysis_summary_response() -> dict[str, Any]:
    return {
        "metadata_additions": {
            "tags": ["时间异常", "悬疑调查", "信任试探"],
            "tone": "紧张、克制",
            "timeline_marker": "暴雨夜午夜前",
        },
        "summaries": {
            "compressed": "林岚确认旧车站发生七分钟时间倒退；周衡看见信封上的齿轮暗记后失态，她又发现其影子没有随画面回退，于是拒绝交信并沿封闭通道继续调查。",
            "one_line": "林岚带着未来来信沿封闭通道追查时间异常。",
            "keywords": ["旧车站", "七分钟", "怀表", "周衡", "封闭通道"],
        },
    }


def chapter_analysis_character_state_response() -> dict[str, Any]:
    return {
        "character_states": {
            "林岚": {
                "location": "旧车站封闭通道",
                "status": "继续调查时间异常",
                "changes": ["确认七分钟倒退真实发生", "决定以行动检验周衡"],
                "emotional_state": "警觉而坚定",
            },
            "周衡": {
                "location": "旧车站封闭通道",
                "status": "身份与动机仍受怀疑",
                "changes": ["异常静止的影子暴露新的疑点"],
                "emotional_state": "克制而紧张",
            },
        }
    }


def chapter_analysis_foreshadowing_response(combined: str) -> dict[str, Any]:
    original_spans = re.findall(r'"original_text_span_id"\s*:\s*"([^"]+)"', combined)
    event_spans = re.findall(r'"source_span_ids"\s*:\s*\[\s*"([^"]+)"', combined)
    if not original_spans or not event_spans:
        raise ValueError("chapter analysis facts do not contain source span references")
    return {
        "foreshadowing": {
            "planted": [
                {
                    "description": "周衡的影子没有随画面回退，暗示他与时间异常存在更深联系。",
                    "original_text_span_id": original_spans[-1],
                    "category": "character_secret",
                    "priority": "high",
                    "related_entities": ["周衡", "实验钟"],
                }
            ],
            "resolved": [],
            "tensions": ["林岚尚未确认周衡是否可信", "封闭通道地下的异常来源仍未揭示"],
        },
        "key_events": [
            {
                "type": "discovery",
                "description": "林岚确认旧车站发生七分钟时间倒退。",
                "importance": "high",
                "involved_characters": ["林岚"],
                "source_span_ids": [event_spans[-1]],
            }
        ],
    }


def optimization_analysis_response(combined: str) -> dict[str, Any]:
    span_match = re.search(
        r'"source_spans"\s*:\s*\[\s*\{\s*"id"\s*:\s*"([^"]+)"',
        combined,
        re.DOTALL,
    )
    if span_match:
        return {
            "thought": "已读取当前正文，可以基于可追溯片段给出局部问题。",
            "public_update": None,
            "action": "final",
            "arguments": {
                "issues": [
                    {
                        "id": "issue-clarity-1",
                        "tool": "edit_article",
                        "arguments": {
                            "evidence_span_id": span_match.group(1),
                            "goal": "把林岚进入地下机房的决定写得更具动作性，并保留对周衡的不信任。",
                        },
                        "description": "结尾决定略偏概述，可增加一个可见动作强化悬疑推进。",
                        "severity": "minor",
                    }
                ]
            },
        }
    return {
        "thought": "先读取当前章节正文和可引用的 source span。",
        "public_update": "正在读取当前章节，定位可安全修改的原文片段。",
        "action": "run_tool",
        "arguments": {
            "tool": "get_project_information",
            "arguments": {"mode": "read", "source": "current_chapter"},
        },
    }


def optimization_patch_response() -> dict[str, Any]:
    return {
        "summary": "为结尾决定补充可见动作，同时保留对周衡的怀疑。",
        "patches": [
            {
                "patch_id": "patch-clarity-1",
                "issue_id": "issue-clarity-1",
                "operation": "replace",
                "old_text": "林岚确认异常不是幻觉，决定先进入地下机房，再判断修表师是否可信。",
                "new_text": "林岚确认异常不是幻觉。她先在墙上划下齿轮暗记，再进入地下机房，决定用行动检验周衡是否可信。",
                "reason": "用可见动作承接决定，强化场景推进和人物戒备。",
                "risk": "low",
            }
        ],
    }


def optimization_chat_response(combined: str) -> dict[str, Any]:
    objects = embedded_json_objects(combined)
    states = [value["loop_state"] for value in objects
              if isinstance(value.get("loop_state"), dict)]
    if states and "observations_ref" in states[-1]:
        records = [value["loop_observation"] for value in objects if isinstance(value.get("loop_observation"), dict)]
        if [record["sequence"] for record in records] != list(range(1, len(records) + 1)):
            raise ValueError("optimization observations are not an intact ordered history")
        observations = [record["value"] for record in records]
        if states[-1]["progress"]["observation_count"] != len(observations):
            raise ValueError("optimization request lost full observations")
    else:
        observations = states[-1].get("observations", []) if states else []
    instruction = re.search(r"# 本次用户指令\s*\n([\s\S]*?)\n# 可用工具", combined)
    current = instruction.group(1) if instruction else ""
    if "A4N_OPTIMIZATION_WAIT" in current:
        time.sleep(3)
    if "A4N_OPTIMIZATION_PLAIN" in current:
        return {"thought": "该问题可直接回答。", "action": "final", "arguments": {"reply": "局部补丁提案需要用户确认后才会写回正文。"}}
    reads = [item["result"] for item in observations
             if item.get("tool") == "get_project_information" and item.get("success") is True]
    if not reads:
        return {"thought": "读取需要修改的局部。", "action": "run_tool", "public_update": "读取本次修改所需的正文片段。",
                "arguments": {"tool": "get_project_information", "arguments": {"mode": "read", "source": "current_chapter", "limit": 128}}}
    proposals = [item["result"] for item in observations
                 if item.get("tool") == "edit_article" and item.get("success") is True]
    if proposals:
        if not proposals[-1].get("patch_set_id") or proposals[-1].get("mode") != "pending_confirmation":
            raise ValueError("optimization tool returned no persisted pending proposal")
        return {"thought": "补丁已持久化，等待用户确认。", "action": "final", "arguments": {"reply": "已生成局部补丁提案，等待用户审阅；正文尚未写回。"}}
    spans = reads[-1]["content"]["source_spans"]
    if not spans:
        raise ValueError("optimization read returned no complete source span")
    original = spans[0]["text"]
    return {"thought": "已有足够的原文证据。", "action": "run_tool", "public_update": "根据已读原文生成局部修改提案。", "arguments": {
        "tool": "edit_article", "arguments": {"summary": "局部表达优化提案", "patches": [{
            "patch_id": "patch-local-1", "operation": "replace", "old_text": original,
            "new_text": original + "她握紧信封，停下脚步。", "reason": "以可见动作承接原文。",
        }]},
    }}


def shooting_span_ids(combined: str) -> list[str]:
    ids = [span_id for span_id, _ in source_span_pairs(combined)]
    ids = list(dict.fromkeys(ids))
    if not ids:
        raise ValueError("shooting script prompt contains no source spans")
    return ids


def shooting_layout_response(combined: str) -> dict[str, Any]:
    indexes = re.findall(r"只细化 narrative_unit_index=(\d+)", combined)
    if not indexes:
        raise UnhandledPromptError("layout prompt contains no target unit index")
    return {
        "schema_version": "screenplay_layout_unit_detail_v1",
        "narrative_unit_index": int(indexes[-1]),
        "transition_contracts": [],
        "title": "封闭通道的七分钟",
        "slugline": "内景 旧车站封闭通道 夜",
        "participants": ["林岚", "周衡"],
        "dramatic_question": "周衡究竟是异常的目击者还是参与者？",
        "scene_objective": "让林岚确认异常并决定继续深入调查。",
        "scene_obstacle": "周衡的影子与回退画面不一致，信任无法建立。",
        "entry_state": "林岚带着逆行怀表在封闭通道入口核对证据。",
        "exit_state": "林岚留下齿轮暗记并进入地下机房。",
    }


def shooting_adaptation_response(combined: str) -> dict[str, Any]:
    context = next((value for value in embedded_json_objects(combined)
                    if isinstance(value.get("unit"), dict) and isinstance(value.get("locked_visual_unit"), dict)), None)
    if context is None:
        raise UnhandledPromptError("adaptation requires a locked director unit")
    unit, locked = context["unit"], context["locked_visual_unit"]
    return {
        "schema_version": "screenplay_unit_adaptation_v1",
        "narrative_unit_index": unit["narrative_unit_index"],
        "performance_facts": [
            {
                "fact_index": index,
                "source_span_ids": [],
                "user_addition_ids": ["director:" + shot["segment_id"]],
                "actor": next(iter(unit["participants"]), None),
                "objective": "用行动验证周衡是否可信。",
                "obstacle": "时间异常与周衡的反常影子都缺少解释。",
                "visible_action": shot["prompt"],
                "reaction": "短暂停顿后收紧怀表。",
                "dialogue_intent": None,
                "entry_state": "警觉地观察周衡。",
                "exit_state": "坚定进入地下机房继续调查。",
                "required": True,
            } for index, shot in enumerate(locked["shots"], 1)
        ],
    }


def shooting_design_scene_template(combined: str) -> dict[str, Any]:
    evidence = source_span_id(combined, "进入地下机房", len(shooting_span_ids(combined)) - 1)
    return {
        "scene_index": 1,
        "scene_identity": "station-passage-to-machine-room",
        "sequence_index": 1,
        "narrative_unit_index": 1,
        "timeline_position": 1,
        "intercut_group_id": None,
        "editing_mode": "continuity",
        "transition_contracts": [],
        "title": "封闭通道的七分钟",
        "slugline": "内景 旧车站封闭通道 夜",
        "location": "旧车站封闭通道",
        "time_of_day": "夜",
        "participants": ["林岚", "周衡"],
        "focal_character": "林岚",
        "source_span_ids": shooting_span_ids(combined),
        "dramatic_question": "周衡究竟是异常的目击者还是参与者？",
        "scene_objective": "让林岚确认异常并决定继续深入调查。",
        "scene_obstacle": "周衡的影子与回退画面不一致，信任无法建立。",
        "relationship_before": "林岚对周衡保持克制怀疑。",
        "relationship_after": "林岚暂不信任周衡，选择独自验证。",
        "audience_knowledge_before": ["旧车站发生七分钟时间倒退。"],
        "audience_knowledge_after": ["周衡可能不受同一时间规则影响。"],
        "withheld_information": ["周衡与实验钟的真实关系。"],
        "viewpoint": {
            "mode": "limited_third_person",
            "anchor_character": "林岚",
            "information_strategy": "只展示林岚可观察到的怀表、广告屏与影子异常。",
        },
        "spatial_continuity": {
            "continuity_intent": "transparent",
            "primary_axis": "封闭通道入口到地下机房方向",
            "screen_direction": "林岚由左向右深入",
            "reorientation_strategy": None,
        },
        "sound_strategy": "第二声钟鸣压住环境底噪，随后留出短暂静默。",
        "entry_state": "林岚在封闭通道入口观察异常。",
        "exit_state": "林岚进入地下机房，周衡留在后方。",
        "shots": [
            {
                "shot_index": 1,
                "shot_id": "ch1-sc1-sh1",
                "title": "划下暗记并进入机房",
                "source_span_ids": [evidence],
                "coverage": "must_show",
                "narrative_purpose": "把怀疑转化为可见行动并推进到下一空间。",
                "subject": "林岚",
                "shot_size": "medium",
                "camera_angle": "eye_level",
                "camera_height": "eye",
                "lens_intent": "保持林岚、墙面暗记与机房入口的空间关系清晰。",
                "camera_movement": "track",
                "composition": "林岚位于前景左侧，齿轮暗记居中，机房入口在纵深右侧。",
                "spatial_anchors": ["封闭通道墙面", "齿轮暗记", "地下机房入口"],
                "blocking": {
                    "start": "林岚停在墙边握住怀表。",
                    "movement": "她划下齿轮暗记后转身走向机房入口。",
                    "end": "她跨入地下机房，周衡留在通道后方。",
                },
                "performance_beats": [
                    {
                        "beat_index": 1,
                        "beat_id": "ch1-sc1-sh1-b1",
                        "source_span_ids": [evidence],
                        "actor": "林岚",
                        "objective": "用行动验证周衡是否可信。",
                        "obstacle": "异常证据仍无法解释。",
                        "visible_action": "划下齿轮暗记并迈入地下机房。",
                        "reaction": "回望周衡一眼后收紧下颌。",
                        "expression": "警觉而坚定",
                        "gaze": "从周衡移向地下机房入口",
                        "pose_or_movement": "收起怀表，身体前倾进入机房",
                        "dialogue_intent": None,
                        "audience_question": "林岚进入机房后会发现什么？",
                        "answer_or_delay": "在本场结尾延后答案。",
                        "knowledge_delta": ["林岚决定不依赖周衡的解释。"],
                        "relationship_delta": "对周衡的信任继续下降。",
                        "spatial_change": "从封闭通道入口进入地下机房。",
                        "entry_state": "警觉观察。",
                        "exit_state": "主动深入调查。",
                        "required": True,
                        "estimated_seconds": 6.0,
                    }
                ],
                "dialogue_cues": [],
                "sound": {
                    "dialogue": [],
                    "environment": ["停电后的通道低频底噪"],
                    "effects": ["金属刻痕声", "怀表合盖声", "脚步声"],
                    "music": ["低频悬疑音持续上扬"],
                    "silence": ["林岚回望周衡时短暂停顿"],
                },
                "camera_start_state": "平视中景锁定林岚与墙面。",
                "camera_end_state": "跟拍至地下机房入口。",
                "start_state": "林岚停在封闭通道墙边。",
                "end_state": "林岚进入地下机房。",
                "transition_in": "承接第二声钟鸣后的静默。",
                "transition_out": "以机房黑暗吞没林岚背影收束。",
                "continuity_requirements": ["怀表始终在林岚右手", "机房入口保持画面右侧"],
                "estimated_natural_seconds": 6.0,
            }
        ],
    }


def shooting_design_input(combined: str) -> tuple[dict[str, Any], dict[str, Any]]:
    for value in embedded_json_objects(combined):
        locked = value.get("locked_visual_unit")
        screenplay = value.get("screenplay_plan_unit")
        if isinstance(locked, dict) and isinstance(screenplay, dict):
            return locked, screenplay
    raise UnhandledPromptError("shooting design prompt contains no locked visual unit")


def shooting_design_response(combined: str) -> dict[str, Any]:
    locked, screenplay = shooting_design_input(combined)
    planned_shots = locked.get("shots")
    facts = screenplay.get("performance_facts")
    if not isinstance(planned_shots, list) or not planned_shots:
        raise UnhandledPromptError("locked visual unit contains no shots")
    if not isinstance(facts, list):
        raise UnhandledPromptError("screenplay plan unit contains no performance facts")

    facts_by_shot: list[list[tuple[dict[str, Any], list[str]]]] = [
        [] for _ in planned_shots
    ]
    for fact in facts:
        if not isinstance(fact, dict) or not isinstance(fact.get("source_span_ids"), list):
            raise UnhandledPromptError("shooting performance fact is invalid")
        for shot_index, planned in enumerate(planned_shots):
            if not isinstance(planned, dict) or not isinstance(planned.get("segment_id"), str):
                raise UnhandledPromptError("locked visual shot is invalid")
            if "director:" + planned["segment_id"] in fact.get("user_addition_ids", []):
                facts_by_shot[shot_index].append((fact, fact["source_span_ids"]))
                break
        else:
            raise UnhandledPromptError(
                f"shooting performance fact {fact.get('fact_index')} has no locked shot"
            )

    response = shooting_design_scene_template(combined)
    template_shot = response["shots"][0]
    template_beat = template_shot["performance_beats"][0]
    participants = screenplay.get("participants")
    if not isinstance(participants, list):
        raise UnhandledPromptError("screenplay plan unit participants are invalid")

    shots = []
    for shot_index, (planned, assigned_facts) in enumerate(
        zip(planned_shots, facts_by_shot), 1
    ):
        start_ms = planned.get("start_ms")
        end_ms = planned.get("end_ms")
        if not isinstance(start_ms, int) or not isinstance(end_ms, int) or start_ms >= end_ms:
            raise UnhandledPromptError("locked visual shot time range is invalid")
        duration_seconds = (end_ms - start_ms) / 1000
        shot = json.loads(json.dumps(template_shot, ensure_ascii=False))
        shot.update(
            {
                "shot_index": shot_index,
                "shot_id": planned["segment_id"],
                "title": planned["title"],
                "source_span_ids": list(dict.fromkeys(span for _, sources in assigned_facts for span in sources)),
                "visual_shot_id": planned["segment_id"],
                "planned_start_ms": start_ms,
                "planned_end_ms": end_ms,
                "user_addition_ids": ["director:" + planned["segment_id"]],
                "visual_constraints": planned,
                "coverage": "must_show",
                "narrative_purpose": planned["prompt"],
                "subject": next(
                    (
                        fact.get("actor")
                        for fact, _ in assigned_facts
                        if isinstance(fact.get("actor"), str)
                    ),
                    participants[0] if participants else planned["title"],
                ),
                "shot_size": planned.get("shot_size") or template_shot["shot_size"],
                "camera_angle": planned.get("camera_angle")
                or template_shot["camera_angle"],
                "camera_height": planned.get("camera_height")
                or template_shot["camera_height"],
                "lens_intent": planned.get("lens_intent") or template_shot["lens_intent"],
                "composition": planned.get("composition") or template_shot["composition"],
                "spatial_anchors": [],
                "blocking": {
                    "start": f"{planned['title']} 的起始状态。",
                    "movement": planned["prompt"],
                    "end": f"{planned['title']} 的结束状态。",
                },
                "continuity_requirements": list(planned.get("continuity_notes", [])),
                "estimated_natural_seconds": duration_seconds,
            }
        )
        beats = []
        for beat_index, (fact, overlap) in enumerate(assigned_facts, 1):
            beat = json.loads(json.dumps(template_beat, ensure_ascii=False))
            beat.update(
                {
                    "beat_index": beat_index,
                    "beat_id": f"{planned['segment_id']}_fact_{fact['fact_index']}",
                    "source_span_ids": overlap,
                    "user_addition_ids": list(fact.get("user_addition_ids", [])),
                    "actor": fact.get("actor"),
                    "objective": fact["objective"],
                    "obstacle": fact["obstacle"],
                    "visible_action": fact["visible_action"],
                    "reaction": fact.get("reaction"),
                    "dialogue_intent": fact.get("dialogue_intent"),
                    "entry_state": fact["entry_state"],
                    "exit_state": fact["exit_state"],
                    "required": bool(fact.get("required")),
                    "estimated_seconds": duration_seconds / len(assigned_facts),
                }
            )
            beats.append(beat)
        shot["performance_beats"] = beats
        shot["dialogue_cues"] = []
        shots.append(shot)

    response.update(
        {
            "scene_identity": screenplay["scene_identity"],
            "sequence_index": screenplay["sequence_index"],
            "narrative_unit_index": screenplay["narrative_unit_index"],
            "timeline_position": screenplay["timeline_position"],
            "intercut_group_id": screenplay.get("intercut_group_id"),
            "editing_mode": screenplay["editing_mode"],
            "transition_contracts": list(screenplay.get("transition_contracts", [])),
            "title": screenplay["title"],
            "slugline": screenplay["slugline"],
            "location": locked.get("location"),
            "time_of_day": locked.get("time_of_day"),
            "participants": list(participants),
            "focal_character": None,
            "source_span_ids": list(screenplay["source_span_ids"]),
            "visual_narrative_unit_id": locked["id"],
            "user_addition_ids": list(locked.get("user_addition_ids", [])),
            "dramatic_question": screenplay["dramatic_question"],
            "scene_objective": screenplay["scene_objective"],
            "scene_obstacle": screenplay["scene_obstacle"],
            "relationship_before": screenplay["entry_state"],
            "relationship_after": screenplay["exit_state"],
            "viewpoint": {
                "mode": "omniscient_objective",
                "anchor_character": None,
                "information_strategy": "只呈现 locked visual unit 已声明的镜头信息。",
            },
            "spatial_continuity": {
                "continuity_intent": "transparent",
                "primary_axis": None,
                "screen_direction": None,
                "reorientation_strategy": None,
            },
            "sound_strategy": "保持 locked visual unit 的声音事件与连续性约束。",
            "entry_state": screenplay["entry_state"],
            "exit_state": screenplay["exit_state"],
            "shots": shots,
        }
    )
    return response


def video_segment_plan_response(combined: str) -> dict[str, Any]:
    payload = json_task_payload(combined)
    shot = payload["current_shot"]
    source_shot_id = str(shot["source_shot_id"])
    beats = [beat for beat in shot.get("action_beats", []) if isinstance(beat, dict)]
    beat_ids = [str(beat["beat_id"]) for beat in beats]
    participants = [str(name) for name in shot.get("participants", [])]
    duration = float(payload["constraints"]["selected_duration_secs"])
    duration_text = f"{duration:g}秒"
    title = str(shot.get("title") or source_shot_id)
    subject = str(shot.get("subject") or "画面主体")
    start_state = str(shot.get("start_state") or shot.get("blocking_start") or "起始状态")
    movement = str(shot.get("blocking_movement") or "保持当前镜头的连续运动")
    end_state = str(shot.get("end_state") or shot.get("blocking_end") or "结束状态")
    anchors = [
        {"anchor": str(anchor), "usage": f"保持 {title} 中的空间关系"}
        for anchor in shot.get("spatial_anchors", [])
        if str(anchor).strip()
    ] or [{"anchor": start_state, "usage": f"锁定 {title} 的起始空间"}]

    if participants:
        actor = next(
            (str(beat.get("actor")) for beat in beats if beat.get("actor") in participants),
            participants[0],
        )
        beat = next((beat for beat in beats if beat.get("actor") == actor), beats[0] if beats else {})
        visible_action = str(beat.get("visible_action") or movement)
        emotion_goals = [
            {
                "character": actor,
                "emotion": "克制而专注",
                "visible_action": visible_action,
            }
        ]
        action_script = [
            {
                "character": actor,
                "body": str(beat.get("pose_or_movement") or "保持与镜头起止状态一致的身体姿态"),
                "movement": visible_action,
                "hands": "双手保持可见且不改变道具关系",
                "gaze": str(beat.get("gaze") or "视线沿当前镜头的叙事目标移动"),
                "timing": f"在{duration_text}内完成当前动作节拍并停在结束状态",
            }
        ]
    else:
        emotion_goals = []
        action_script = []

    segment = {
                "source_shot_id": source_shot_id,
                "covered_beat_ids": beat_ids,
                "title": title,
                "scene_summary": f"{subject}在{duration_text}内从{start_state}经过{movement}到达{end_state}。",
                "blocking_summary": movement,
                "spatial_anchors": anchors,
                "emotion_action_goals": emotion_goals,
                "video_intent": {
                    "hook": f"以{movement}让观众读清当前镜头的关键变化",
                    "audience_effect": "保持当前镜头的叙事信息和空间连续性",
                    "rhythm": f"在{duration_text}内保持单一、可读的连续动作",
                    "visual_priority": f"优先呈现{start_state}、动作桥和{end_state}",
                },
                "emotional_arc": {
                    "start": start_state,
                    "turn": movement,
                    "end": end_state,
                    "function": str(shot.get("narrative_purpose") or "推进当前镜头"),
                },
                "performance_plan": {
                    "movement_path": movement,
                    "body_performance": "保持起止状态之间的连续身体变化",
                    "gaze_and_hands": "保持视线、手部和道具关系稳定",
                    "pause_or_reaction": "在结束状态保留短暂停顿",
                    "camera_relationship": str(shot.get("composition") or "保持当前镜头构图"),
                    "time_blocks": [
                        {
                            "start_sec": 0,
                            "end_sec": duration,
                            "camera": str(shot.get("camera_start_state") or "保持当前机位和运动方向"),
                            "subject_action": movement,
                            "continuity_target": end_state,
                        }
                    ],
                    "action_script": action_script,
                },
                "material_coverage": {
                    "required_entities": [subject],
                    "missing_or_weak_references": [],
                    "risk_note": "当前镜头上下文已提供所需素材约束",
                },
    }
    segment_count = max(1, int(shot.get("minimum_readable_segment_count") or 1))
    segments = []
    for index in range(segment_count):
        item = dict(segment)
        if segment_count > 1:
            item["title"] = f"{title} · 阶段 {index + 1}"
        segments.append(item)
    return {"segments": segments}


def video_static_frame_response(combined: str) -> dict[str, Any]:
    roles = re.findall(r'"role"\s*:\s*"(chain_start|shared_boundary|chain_end)"', combined)
    is_end = bool(roles and roles[-1] == "chain_end")
    if is_end:
        title = "机房入口尾帧"
        scene_summary = "林岚已经跨入地下机房入口，通道后的周衡仍停在原位。"
        position = "地下机房门槛内侧"
        facing = "面向机房深处，侧脸回望通道"
        pose = "右手收紧怀表，重心前移后稳定停步"
        frame_prompt = (
            "【参考图约束】以林岚的深色风衣、旧怀表和旧车站封闭通道文字设定作为一致性基准。"
            "【画面主体】林岚已经跨入地下机房门槛内侧，身体朝向右侧纵深，侧脸短暂回望通道，右手合拢旧怀表贴近身体，神情坚定。"
            "【空间与道具】齿轮暗记留在画面左后方墙面，地下机房入口位于右侧纵深，周衡停在更远的通道后方，不互换左右关系。"
            "【构图光线风格】平视中景，冷绿色机房指示灯从右前方勾出轮廓，通道暖灰弱光从左后方衰减，都市时间悬疑电影感。"
            "【一致性与避免项】保持林岚脸部、风衣、怀表右手状态、暗记位置和光源方向连续，画面只表现稳定尾态。"
        )
        anchors = ["齿轮暗记", "地下机房入口"]
        intent = "冷绿轮廓光把林岚从通道背景中分离，强化主动进入未知空间的压力。"
    else:
        title = "通道墙边首帧"
        scene_summary = "林岚停在封闭通道墙边，握住怀表观察周衡与机房入口。"
        position = "封闭通道墙面左侧"
        facing = "先看墙面暗记位置，再看画面右侧机房入口"
        pose = "肩颈收紧，右手握住旧怀表"
        frame_prompt = (
            "【参考图约束】以林岚的深色风衣、旧怀表和旧车站封闭通道文字设定作为一致性基准。"
            "【画面主体】林岚站在画面左侧墙边，肩颈略收紧，右手握住旧怀表，视线从空白墙面移向画面右侧纵深的地下机房入口，表情警觉。"
            "【空间与道具】墙面位于人物近侧，准备刻下暗记的位置居中，地下机房入口保持在右侧纵深，周衡停在通道后方。"
            "【构图光线风格】平视中景保持人物、墙面和入口同框，冷灰顶灯从左上方照亮风衣轮廓，机房内有微弱冷绿色指示灯，都市时间悬疑电影感。"
            "【一致性与避免项】保持林岚脸部、风衣、怀表位置、人物左右关系和光源方向稳定，不提前表现进入机房后的动作。"
        )
        anchors = ["封闭通道墙面", "地下机房入口"]
        intent = "左上冷灰光压住通道空间，让机房入口的微弱绿光形成诱导与威胁。"
    return {
        "frames": [
            {
                "title": title,
                "scene_summary": scene_summary,
                "frame_prompt": frame_prompt,
                "blocking_state": {
                    "characters": [
                        {
                            "name": "林岚",
                            "position": position,
                            "facing": facing,
                            "pose": pose,
                        }
                    ],
                    "spatial_anchors": anchors,
                    "do_not_change": ["林岚始终位于通道左侧到机房右侧纵深的同一运动轴线上"],
                },
                "lighting_state": {
                    "time_of_day": "夜",
                    "key_light": "通道冷灰顶灯与机房冷绿指示灯",
                    "direction": "冷灰光从左上方，冷绿光从右前方",
                    "shadow": "人物身后保留柔和长阴影",
                    "intent": intent,
                },
                "negative_prompt": "低清晰度、画面模糊、人体结构扭曲、多余肢体、换脸、怀表消失、人物左右关系颠倒、机房入口移位、光影方向跳变、字幕、水印",
            }
        ]
    }


def video_storyboard_frame_response() -> dict[str, Any]:
    return {
        "frames": [
            {
                "title": "暗记与机房入口三格分镜板",
                "scene_summary": "林岚从警觉观察、刻下暗记到坚定进入地下机房的连续动作。",
                "storyboard_panels": [
                    {
                        "panel_index": 1,
                        "moment": "起始状态",
                        "visual_goal": "看清林岚、墙面和机房入口的固定空间关系",
                        "composition": "平视中景，林岚在左、墙面居中、机房入口在右侧纵深",
                        "continuity_note": "深色风衣、右手怀表与左上冷灰光保持不变",
                    },
                    {
                        "panel_index": 2,
                        "moment": "核心动作",
                        "visual_goal": "看清林岚刻下齿轮暗记并回望周衡的停顿",
                        "composition": "中景轻微靠近，暗记居中，周衡留在后方虚化空间",
                        "continuity_note": "人物不越轴，怀表始终在右手，机房入口仍在右侧",
                    },
                    {
                        "panel_index": 3,
                        "moment": "结尾状态",
                        "visual_goal": "看清林岚跨入机房并被冷绿轮廓光包围",
                        "composition": "中景沿右侧纵深收束，林岚背影位于门槛内侧",
                        "continuity_note": "暗记留在左后方，周衡留在通道后方，风衣和光源连续",
                    },
                ],
                "frame_prompt": (
                    "【参考图约束】以林岚深色风衣、旧怀表和封闭通道文字设定锁定人物与场景一致性。"
                    "【画面主体】制作一张横向三格动态分镜板：第 1 格，林岚站在左侧墙边握住怀表观察右侧机房入口；"
                    "第 2 格，林岚刻下居中的齿轮暗记并停半拍回望后方周衡；第 3 格，林岚收紧怀表、重心前移，跨入右侧纵深的地下机房。"
                    "【空间与道具】三格都保持墙面在中部、机房入口在右侧、周衡在通道后方，怀表始终位于林岚右手。"
                    "【构图光线风格】平视中景逐格轻微推进，左上冷灰顶灯连续，机房冷绿指示灯在末格增强，都市时间悬疑电影感。"
                    "【一致性与避免项】保持林岚脸部、风衣、怀表、运动轴线和光源方向连续，分镜板只使用三格且不要文字编号、对白框或漫画气泡。"
                ),
                "blocking_state": {
                    "characters": [
                        {
                            "name": "林岚",
                            "position": "从通道左侧墙边移动到右侧纵深的机房门槛",
                            "facing": "由墙面与周衡转向机房深处",
                            "pose": "握表观察、刻下暗记、迈步进入",
                        }
                    ],
                    "spatial_anchors": ["封闭通道墙面", "齿轮暗记", "地下机房入口"],
                    "do_not_change": ["人物不越轴，机房入口始终在画面右侧"],
                },
                "lighting_state": {
                    "time_of_day": "夜",
                    "key_light": "通道冷灰顶灯",
                    "direction": "左上方主光，右前方机房冷绿辅光",
                    "shadow": "三格阴影方向连续并逐步加深",
                    "intent": "光线从观察阶段的压抑过渡到进入机房时的未知威胁",
                },
                "negative_prompt": "低清晰度、人体结构扭曲、多余肢体、换脸、服装变化、怀表消失、空间跳变、光影不连续、分屏数量错误、字幕、水印、乱码文字",
            }
        ]
    }


def video_prompt_response() -> dict[str, Any]:
    visual_prompt = "\n".join(
        [
            "[主体定义与素材绑定]\n林岚保持28岁女性、深色风衣和右手旧怀表的一致外观；封闭通道墙面、齿轮暗记和地下机房入口是固定空间锚点。",
            "[片段内容以及前因后果]\n时间异常后，林岚不再等待周衡解释，她要留下可核验的暗记并独自进入地下机房，压力来自未知空间和对周衡的不信任。",
            "[镜头分镜]\n镜头 1：平视中景固定观察，林岚在画面左侧墙边刻下齿轮暗记，右手合拢怀表后停半拍回望通道后方。\n镜头 2：镜头沿画面右侧纵深缓慢跟随，林岚肩颈由紧绷转为稳定，重心前移，以克制步幅跨入地下机房入口。",
            "[画面风格描述]\n都市时间悬疑电影感，冷灰顶灯与机房冷绿指示灯形成低饱和对比，平视中景和纵深构图强调孤立与未知。",
            "[连续性与结尾状态]\n怀表始终在林岚右手，暗记留在左后方墙面，机房入口保持右侧；结尾林岚背影停在门槛内侧，周衡仍在通道后方。",
            "[限制与避免项]\n不要配背景音乐，不要字幕、水印、换脸、服装变化、怀表消失、人物越轴、空间跳变、光源方向突变或过快炫技运镜。",
        ]
    )
    motion_prompt = "\n".join(
        [
            "[运镜与动作]\n全段只采用平视中景的缓慢纵深跟随：先固定看清刻下暗记和停顿，再随林岚从左侧墙边转向右侧机房入口；动作包含合拢怀表、回望、重心前移和跨过门槛。",
            "[对白与表演]\n本段无对白。林岚先压住呼吸并收紧肩颈，刻完暗记后停半拍看向周衡，随后下颌收紧、步幅稳定地进入机房；周衡留在后方不追随。",
        ]
    )
    return {
        "shots": [
            {
                "title": "暗记之后进入机房",
                "scene_summary": "林岚留下齿轮暗记并从警觉转为坚定，独自进入地下机房。",
                "visual_prompt": visual_prompt,
                "motion_prompt": motion_prompt,
                "negative_prompt": "避免人物越轴、怀表换手、机房入口移位、光影不连续和动作跳帧",
                "dialogue_cues": [],
            }
        ]
    }


def chat_video_prompt_response() -> dict[str, Any]:
    return {
        "shots": [
            {
                "title": "暗记之后进入机房",
                "scene_summary": "林岚留下齿轮暗记并独自进入地下机房。",
                "chat_prompt": video_prompt_response()["shots"][0]["visual_prompt"],
                "negative_prompt": "避免人物越轴、怀表换手、机房入口移位、光影不连续和动作跳帧",
            }
        ]
    }


def current_task_prompt_name(combined: str) -> str | None:
    matches = re.findall(
        r'"task"\s*:\s*\{[^{}]*"prompt_name"\s*:\s*"([^"]+)"',
        combined,
    )
    return matches[-1] if matches else None


def json_task_payload(combined: str) -> dict[str, Any]:
    marker = "\n\n输入：\n"
    marker_index = combined.rfind(marker)
    if marker_index < 0:
        raise ValueError("prompt contains no JSON task payload")
    payload = json.loads(combined[marker_index + len(marker) :])
    if not isinstance(payload, dict):
        raise ValueError("JSON task payload must be an object")
    return payload


def galgame_beat_ref(scene: dict[str, Any]) -> dict[str, int]:
    beat = scene["beats"][0]
    return {
        "scene_index": int(scene["index"]),
        "shot_index": int(beat["shot_index"]),
        "beat_index": int(beat["beat_index"]),
    }


def galgame_shared_step(combined: str, name: str) -> dict[str, Any]:
    for value in embedded_json_objects(combined):
        step = value.get(name)
        if isinstance(step, dict) and "$ref" not in step:
            return step
    raise UnhandledPromptError(f"galgame prompt contains no shared {name}")


def galgame_branch_design_response(combined: str) -> dict[str, Any]:
    scene = galgame_shared_step(combined, "shooting_script_projection")["scenes"][0]
    beat_ref = galgame_beat_ref(scene)
    return_scene_index = int(scene["index"])
    return {
        "choices": [
            {
                "after_beat_ref": beat_ref,
                "question": "林岚要怎样验证眼前的异常？",
                "options": [
                    {
                        "branch_id": "inspect_watch",
                        "option_text": "检查逆行怀表",
                        "branch_title": "怀表里的七分钟",
                        "branch_objective": "确认时间倒退不是错觉",
                        "dramatic_action": "林岚贴近表盘核对逆行秒针",
                        "reaction": "周衡下意识收紧工具箱提手",
                        "player_knowledge": ["怀表仍在逆行，异常具有可重复证据"],
                        "returns_to_scene_index": return_scene_index,
                    },
                    {
                        "branch_id": "question_zhou",
                        "option_text": "追问齿轮暗记",
                        "branch_title": "周衡的停顿",
                        "branch_objective": "确认周衡与十年前事故的关系",
                        "dramatic_action": "林岚把信封暗记转向周衡",
                        "reaction": "周衡避开视线，回答前明显停顿",
                        "player_knowledge": ["周衡认识齿轮暗记并隐瞒了信息"],
                        "returns_to_scene_index": return_scene_index,
                    },
                ],
            }
        ]
    }


def galgame_source_span_id(
    source_spans: list[dict[str, Any]], phrase: str, fallback_index: int
) -> str:
    for span in source_spans:
        if phrase in str(span.get("text", "")):
            return str(span["id"])
    return str(source_spans[fallback_index]["id"])


def galgame_script_adaptation_response(combined: str) -> dict[str, Any]:
    task_input = json_task_payload(combined)["input"]
    scene = galgame_shared_step(combined, "shooting_script_projection")["scenes"][0]
    branch_design = galgame_shared_step(combined, "branch_design")
    beat_ref = galgame_beat_ref(scene)
    source_spans = task_input["chapters"][0]["source_spans"]
    first_span = galgame_source_span_id(source_spans, "雨", 0)
    zhou_span = galgame_source_span_id(source_spans, "十年前", min(3, len(source_spans) - 1))
    watch_span = galgame_source_span_id(source_spans, "怀表", 0)
    final_span = galgame_source_span_id(source_spans, "十八分钟", len(source_spans) - 1)
    branch_lines = []
    for choice in branch_design["choices"]:
        for option in choice["options"]:
            if option["branch_id"] == "inspect_watch":
                action = "林岚把怀表贴近广告屏的冷光，逆行秒针清晰走过一格。"
                dialogue = "它没有停，倒退还在继续。"
                speaker = "林岚"
                evidence = watch_span
            else:
                action = "林岚把信封上的齿轮暗记转向周衡，挡住他后退的路线。"
                dialogue = "这个记号和十年前到底有什么关系？"
                speaker = "林岚"
                evidence = zhou_span
            branch_lines.append(
                {
                    "branch_id": option["branch_id"],
                    "lines": [
                        {
                            "index": 1,
                            "beat_ref": beat_ref,
                            "line_type": "stage_direction",
                            "speaker_name": "",
                            "text": action,
                            "source_span_ids": [evidence],
                        },
                        {
                            "index": 2,
                            "beat_ref": beat_ref,
                            "line_type": "dialogue",
                            "speaker_name": speaker,
                            "text": dialogue,
                            "source_span_ids": [evidence],
                        },
                    ],
                }
            )
    return {
        "scene_lines": [
            {
                "scene_index": int(scene["index"]),
                "lines": [
                    {
                        "index": 1,
                        "beat_ref": beat_ref,
                        "line_type": "narration",
                        "speaker_name": "",
                        "text": "停电后的封闭通道只剩广告屏回退的冷光。",
                        "source_span_ids": [first_span],
                    },
                    {
                        "index": 2,
                        "beat_ref": beat_ref,
                        "line_type": "dialogue",
                        "speaker_name": "周衡",
                        "text": "这个记号十年前就该被烧掉了。",
                        "source_span_ids": [zhou_span],
                    },
                    {
                        "index": 3,
                        "beat_ref": beat_ref,
                        "line_type": "monologue",
                        "speaker_name": "林岚",
                        "text": "第一个预告已经发生，我只相信能重复验证的证据。",
                        "source_span_ids": [watch_span],
                    },
                    {
                        "index": 4,
                        "beat_ref": beat_ref,
                        "line_type": "stage_direction",
                        "speaker_name": "",
                        "text": "林岚在墙上划下齿轮暗记，合上怀表走向地下机房。",
                        "source_span_ids": [final_span],
                    },
                ],
            }
        ],
        "branch_scene_lines": branch_lines,
    }


def galgame_visible_cast(profile_by_name: dict[str, int]) -> list[dict[str, Any]]:
    cast = []
    if "林岚" in profile_by_name:
        cast.append(
            {
                "character_profile_id": profile_by_name["林岚"],
                "expression": "serious",
                "pose": "holding_pocket_watch",
                "position": "left",
                "facing": "right",
                "layer": "front",
            }
        )
    if "周衡" in profile_by_name:
        cast.append(
            {
                "character_profile_id": profile_by_name["周衡"],
                "expression": "neutral",
                "pose": "holding_toolbox",
                "position": "right",
                "facing": "left",
                "layer": "middle",
            }
        )
    return cast


def galgame_staged_line(
    line: dict[str, Any], profile_by_name: dict[str, int], enter: bool
) -> dict[str, Any]:
    speaker_name = str(line.get("speaker_name") or "")
    speaker_profile_id = profile_by_name.get(speaker_name)
    visible_cast = galgame_visible_cast(profile_by_name)
    visible_ids = [item["character_profile_id"] for item in visible_cast]
    focus = speaker_profile_id
    if focus is None and "林岚" in profile_by_name:
        focus = profile_by_name["林岚"]
    return {
        "index": line["index"],
        "beat_ref": line["beat_ref"],
        "line_type": line["line_type"],
        "speaker_profile_id": speaker_profile_id,
        "speaker_name": speaker_name,
        "text": line["text"],
        "presentation": {
            "schema_version": "galgame_line_presentation_v2",
            "visible_cast": visible_cast,
            "enter": visible_ids if enter else [],
            "exit": [],
            "focus_character_profile_id": focus,
            "hold": line["line_type"] in {"monologue", "stage_direction"},
            "player_knowledge": [f"演出后确认：{line['text']}"],
        },
        "source_span_ids": line["source_span_ids"],
    }


def galgame_character_staging_response(combined: str) -> dict[str, Any]:
    task_input = json_task_payload(combined)["input"]
    draft = galgame_shared_step(combined, "script_adaptation")
    profile_by_name = {
        str(profile["name"]): int(profile["id"])
        for profile in task_input["speaker_profiles"]
    }
    return {
        "scene_lines": [
            {
                "scene_index": scene["scene_index"],
                "lines": [
                    galgame_staged_line(line, profile_by_name, index == 0)
                    for index, line in enumerate(scene["lines"])
                ],
            }
            for scene in draft["scene_lines"]
        ],
        "branch_scene_lines": [
            {
                "branch_id": branch["branch_id"],
                "lines": [
                    galgame_staged_line(line, profile_by_name, index == 0)
                    for index, line in enumerate(branch["lines"])
                ],
            }
            for branch in draft["branch_scene_lines"]
        ],
    }


def galgame_visual_planning_response(combined: str) -> dict[str, Any]:
    scene = galgame_shared_step(combined, "shooting_script_projection")["scenes"][0]
    staged_scene = galgame_shared_step(combined, "character_staging")["scene_lines"][0]
    source_span_ids = scene["source_span_ids"]
    cg_line = staged_scene["lines"][-1]
    return {
        "backgrounds": [
            {
                "scene_index": scene["index"],
                "background_note": "旧车站封闭通道，夜间停电，冷灰广告屏反光与地下机房冷绿指示灯形成纵深，墙面保留齿轮暗记。",
                "scene_profile_id": None,
                "source_span_ids": [source_span_ids[0]],
            }
        ],
        "event_cgs": [
            {
                "scene_index": scene["index"],
                "line_index": cg_line["index"],
                "cg_note": "林岚合上逆行怀表，在墙面留下齿轮暗记后跨入地下机房；周衡提着工具箱停在后方。",
                "importance": "high",
                "source_span_ids": cg_line["source_span_ids"],
            }
        ],
    }


def manga_beat_ref(combined: str) -> dict[str, int]:
    for value in reversed(embedded_json_objects(combined)):
        events = value.get("events")
        if not isinstance(events, list):
            continue
        for event in events:
            beat_ref = event.get("beat_ref") if isinstance(event, dict) else None
            if isinstance(beat_ref, dict) and all(
                isinstance(beat_ref.get(field), int)
                for field in ("scene_index", "shot_index", "beat_index")
            ):
                return {
                    "scene_index": beat_ref["scene_index"],
                    "shot_index": beat_ref["shot_index"],
                    "beat_index": beat_ref["beat_index"],
                }
    matches = re.findall(
        r'"beat_ref"\s*:\s*\{\s*"scene_index"\s*:\s*(\d+)\s*,\s*'
        r'"shot_index"\s*:\s*(\d+)\s*,\s*"beat_index"\s*:\s*(\d+)\s*\}',
        combined,
    )
    scene_index, shot_index, beat_index = matches[-1] if matches else ("1", "1", "1")
    return {
        "scene_index": int(scene_index),
        "shot_index": int(shot_index),
        "beat_index": int(beat_index),
    }


def manga_available_source_span_ids(combined: str) -> list[str]:
    matches = re.findall(
        r'"available_source_span_ids"\s*:\s*\[(.*?)\]', combined, re.DOTALL
    )
    source_span_ids = re.findall(r'"(ch\d+:s\d+:f\d+)"', matches[-1]) if matches else []
    if not source_span_ids:
        raise UnhandledPromptError("manga prompt contains no available source span ids")
    return source_span_ids


def manga_character_binding_response(combined: str) -> dict[str, Any]:
    return {
        "characters": {
            "林岚": {
                "name": "林岚",
                "appearance": "28岁青年女性，黑色短发，深色风衣，右手常握旧怀表",
                "personality": "克制警觉，遇到异常时先核验证据再行动",
                "role": "protagonist",
                "gender": "female",
                "age_description": "28岁青年女性",
                "source_span_ids": [source_span_id(combined, "林岚")],
            },
            "周衡": {
                "name": "周衡",
                "appearance": "青年男性修表师，深色工作夹克，提旧工具箱",
                "personality": "表面从容，提及十年前的齿轮暗记时明显回避",
                "role": "supporting",
                "gender": "male",
                "age_description": "约30岁青年男性",
                "source_span_ids": [source_span_id(combined, "周衡", 1)],
            },
        }
    }


def manga_speech_selection_response(combined: str) -> dict[str, Any]:
    beat_ref = manga_beat_ref(combined)
    return {
        "dialogues": [
            {
                "index": 0,
                "speaker": "周衡",
                "content": "这个记号十年前就该被烧掉了。",
                "source_span_id": source_span_id(combined, "这个记号十年前"),
                "emotion": "压低声音的警惕",
                "target": "林岚",
                "beat_ref": beat_ref,
                "is_internal": False,
                "bubble_type": "normal",
            },
            {
                "index": 1,
                "speaker": "林岚",
                "content": "第一个预告已经发生。",
                "source_span_id": source_span_id(combined, "第一个预告", -1),
                "emotion": "克制而坚定",
                "target": None,
                "beat_ref": beat_ref,
                "is_internal": True,
                "bubble_type": "thought",
            },
        ],
        "narrations": [
            {
                "index": 0,
                "content": "距离午夜只剩十八分钟。",
                "source_span_id": source_span_id(combined, "距离午夜", -1),
                "narration_type": "time",
                "beat_ref": beat_ref,
                "position": "top",
            }
        ],
    }


def manga_visual_anchor_response(combined: str) -> dict[str, Any]:
    source_ids = list(
        dict.fromkeys(
            [
                source_span_id(combined, "旧怀表"),
                source_span_id(combined, "封闭通道", -1),
            ]
        )
    )
    return {
        "scenes": [
            {
                "index": 0,
                "location": "旧车站封闭通道",
                "location_en": "sealed station passage",
                "time_of_day": "夜",
                "atmosphere": "停电后的通道把林岚与周衡拉开距离，机房入口形成压迫性的黑暗纵深",
                "weather": None,
                "lighting": "冷灰顶灯间歇闪烁，地下机房入口透出低亮冷绿指示灯",
                "indoor_outdoor": "indoor",
                "description": "前景是林岚握紧的旧怀表，中景墙面保留齿轮暗记，周衡停在后方，机房入口位于右侧纵深",
                "beat_refs": [manga_beat_ref(combined)],
                "source_span_ids": source_ids,
            }
        ]
    }


def manga_item_response(combined: str) -> dict[str, Any]:
    source_span_id = manga_available_source_span_ids(combined)[0]
    return {
        "items": [
            {
                "name": "旧怀表",
                "description": "林岚随身携带、秒针曾逆行一圈的旧怀表",
                "description_en": "worn brass pocket watch with a counterclockwise second hand",
                "importance": "prop",
                "first_appearance_beat_ref": manga_beat_ref(combined),
                "visual_features": "黄铜外壳和磨损刻度必须连续一致；被林岚握紧时成为读者视线焦点并强化她的警觉",
                "source_span_ids": [source_span_id],
            }
        ]
    }


def manga_synthesis_response() -> dict[str, Any]:
    return {
        "chapter_summary": "林岚用逆行怀表确认旧车站发生七分钟倒退，并从周衡失控的反应中察觉他隐瞒着十年前的秘密。她拒绝交出未来来信，留下齿轮暗记后独自进入地下机房。",
        "mood_progression": ["警觉观察", "异常坐实", "关系失衡", "主动深入"],
        "total_estimated_pages": 5,
    }


def manga_page_plan_response(combined: str) -> dict[str, Any]:
    script_ids = re.findall(r'"shooting_script_id"\s*:\s*(\d+)', combined)
    script_revisions = re.findall(r'"shooting_script_revision"\s*:\s*(\d+)', combined)
    if not script_ids or not script_revisions:
        raise ValueError("manga page planning prompt contains no shooting script identity")

    beat_ref = manga_beat_ref(combined)
    states = [
        "林岚在封闭通道入口核对逆行怀表，仍与周衡保持距离。",
        "林岚确认七分钟倒退并把怀表收回掌心。",
        "周衡看见齿轮暗记后失去从容，双方互相试探。",
        "林岚拒绝交出未来来信，决定只相信可验证的证据。",
        "林岚在墙上留下齿轮暗记并转向地下机房。",
        "林岚跨入地下机房，周衡仍停在通道后方。",
    ]
    page_specs = [
        ("opening", "slow", "high", "establish", "wide", "eye_level"),
        ("setup", "medium", "high", "dialogue", "medium", "over_shoulder"),
        ("rising", "slow", "critical", "reaction", "close_up", "profile"),
        ("climax", "fast", "critical", "revelation", "close_up", "low"),
        ("resolution", "medium", "high", "action", "wide", "eye_level"),
    ]
    text_layers = [
        [{"layer_index": 1, "kind": "narration", "speaker": None, "text": "距离午夜只剩十八分钟。", "placement": "top_left", "avoid_covering": ["怀表", "通道入口"]}],
        [{"layer_index": 1, "kind": "dialogue", "speaker": "周衡", "text": "这个记号十年前就该被烧掉了。", "placement": "top_right", "avoid_covering": ["周衡的眼睛", "齿轮暗记"]}],
        [{"layer_index": 1, "kind": "thought", "speaker": "林岚", "text": "第一个预告已经发生。", "placement": "upper_center", "avoid_covering": ["林岚的眼睛", "未来来信"]}],
        [{"layer_index": 1, "kind": "sfx", "speaker": None, "text": "咚——", "placement": "center_right", "avoid_covering": ["机房入口", "林岚的手"]}],
        [],
    ]
    actions = [
        "林岚低头核对逆行的旧怀表，周衡停在通道后方。",
        "周衡盯住信封角落的齿轮暗记，肩膀瞬间僵住。",
        "林岚收拢未来来信，视线越过周衡看向地下机房。",
        "第二声钟鸣压下环境声，林岚在墙面划出齿轮暗记。",
        "林岚合上怀表并跨过机房门槛，周衡没有跟上。",
    ]
    reactions = [
        "周衡避开林岚的视线。",
        "林岚握紧信封，没有递出。",
        "周衡的影子仍停在原位。",
        "周衡在后景停顿，没有上前阻拦。",
        "林岚回望一眼后收紧下颌。",
    ]

    pages = []
    for index, spec in enumerate(page_specs):
        role, pacing, importance, function, shot_scale, camera_angle = spec
        page_number = index + 1
        pages.append(
            {
                "page_number": page_number,
                "beat_refs": [beat_ref],
                "role": role,
                "pacing": pacing,
                "importance": importance,
                "reading_flow": "z_path",
                "page_goal": f"第{page_number}页推进林岚从确认异常到主动进入机房的决定。",
                "visual_thesis": "用人物距离、怀表与机房入口的纵深关系表现信任断裂和行动压力。",
                "entry_state": states[index],
                "exit_state": states[index + 1],
                "dominant_panel_number": 1,
                "panels": [
                    {
                        "panel_number": 1,
                        "beat_refs": [beat_ref],
                        "function": function,
                        "size": "dominant",
                        "shot_scale": shot_scale,
                        "camera_angle": camera_angle,
                        "participants": ["林岚", "周衡"],
                        "composition": "林岚位于前景左侧，怀表或信件形成视觉焦点；周衡处于中景，地下机房入口在右侧纵深，通道线条把视线导向出口。",
                        "observable_action": actions[index],
                        "reaction": reactions[index],
                        "spatial_relationship": "林岚靠近左侧墙面，周衡始终落后半个通道，机房入口位于两人视线前方右侧。",
                        "continuity_from_previous": None if index == 0 else f"承接上一页状态：{states[index]}",
                        "text_layers": text_layers[index],
                    }
                ],
            }
        )
    return {
        "schema_version": "manga_page_plan_v3",
        "shooting_script_id": int(script_ids[-1]),
        "shooting_script_revision": int(script_revisions[-1]),
        "pages": pages,
    }


def manga_drawing_prompt_response(combined: str) -> dict[str, Any]:
    page_numbers = re.findall(r'"page_number"\s*:\s*(\d+)', combined)
    page_number = int(page_numbers[-1]) if page_numbers else 1
    return {
        "schema_version": "manga_page_drawing_prompt_v2",
        "page_number": page_number,
        "image_prompt": (
            f"完整中文漫画第{page_number}页，按 Z 字阅读流绘制一个非均分的 dominant 主画格。"
            "严格保留第1格的角色、动作、反应、前中后景和文字层：林岚位于前景左侧，周衡在中景后方，"
            "旧车站地下机房入口位于右侧纵深；保持林岚深色风衣、右手旧怀表、周衡工作夹克和工具箱全页一致。"
            "使用清晰画格边框与 gutter，从左上建立阅读入口，右下作为阅读出口；文字按计划位置排布，"
            "不得遮挡眼睛、手部、怀表、齿轮暗记和机房入口，不新增或改写台词。"
        ),
    }


def manga_prompt_optimization_response(combined: str) -> dict[str, str]:
    prompt_values = []
    for encoded in re.findall(r'"image_prompt"\s*:\s*("(?:\\.|[^"\\])*")', combined):
        try:
            prompt_values.append(json.loads(encoded))
        except json.JSONDecodeError:
            continue
    current_prompt = max(prompt_values, key=len) if prompt_values else "完整中文漫画页"
    return {
        "image_prompt": current_prompt
        + "\n优化要求落实：强化冷灰顶灯与冷绿指示灯的前中后景层次，保持单一主画格和清晰 gutter，压缩重复形容词；原有剧情、角色、构图、参考图、动作反应与全部文字层保持不变。"
    }


def chapter_visual_plan_fixture(scene_profile_id: int) -> dict[str, Any]:
    return {
        "schema_version": "chapter_visual_plan_v3",
        "frame": {"width": 1024, "height": 1024},
        "points": [
            {"id": point_id, "at_ms": at_ms, "label": point_id,
             "image_id": None, "description": "车站中的分镜画面", "reference_image_ids": []}
            for point_id, at_ms in [("start", 0), ("end", 1000)]
        ],
        "segments": [{"id": "shot_fixture", "title": "Fixture shot", "start_point_id": "start", "end_point_id": "end",
                      "prompt": "林岚停在通道入口，抬头观察墙上的钟。", "scene_binding": {"kind": "scene", "scene_profile_id": scene_profile_id},
                      "use_end_point_image": False}],
    }


def embedding_for(text: str, dimension: int) -> list[float]:
    digest = hashlib.sha256(text.encode("utf-8")).digest()
    return [round((digest[index % len(digest)] - 127.5) / 127.5, 6) for index in range(dimension)]


def embedded_json_objects(text: str) -> list[dict[str, Any]]:
    decoder = json.JSONDecoder()
    values = []
    for index, character in enumerate(text):
        if character != "{":
            continue
        try:
            value, _ = decoder.raw_decode(text, index)
        except json.JSONDecodeError:
            continue
        if isinstance(value, dict):
            values.append(value)
    return values


def reverse_source_context(combined: str) -> dict[str, Any]:
    for value in embedded_json_objects(combined):
        context = value.get("source_context")
        if isinstance(context, dict):
            return context
    return {}


def reverse_blueprint_response() -> dict[str, Any]:
    story = blueprint_response("story_core")
    return {
        **story,
        "world_setting": blueprint_response("world")["world_setting"],
        "characters": blueprint_response("characters")["characters"],
        "relationships": blueprint_response("relationships")["relationships"],
    }


def reverse_outline_response(combined: str) -> dict[str, Any]:
    summaries = reverse_source_context(combined).get("summaries")
    if not isinstance(summaries, list) or not summaries:
        summaries = [{"chapter_number": 1, "title": "雨夜来信"}]
    outlines = []
    for index, item in enumerate(summaries, 1):
        item = item if isinstance(item, dict) else {}
        chapter_number = item.get("chapter_number", index)
        title = item.get("title") or f"第{chapter_number}章"
        outlines.append(
            {
                "chapter_number": chapter_number,
                "title": title,
                "summary": f"第{chapter_number}章围绕{title}推进：林岚核验时间异常线索，与周衡共同追查旧车站钟楼，并在新的证据中逼近未来来信的真相。",
            }
        )
    return {"outlines": outlines}


def reverse_part_outline_response(combined: str) -> dict[str, Any]:
    context = reverse_source_context(combined)
    outlines = context.get("chapter_outlines")
    chapter_numbers = [
        item.get("chapter_number")
        for item in outlines or []
        if isinstance(item, dict) and isinstance(item.get("chapter_number"), int)
    ]
    total = context.get("total_chapters")
    if not isinstance(total, int) or total <= 0:
        total = max(chapter_numbers, default=1)
    parts = []
    for part_number, start in enumerate(range(1, total + 1, 25), 1):
        end = min(start + 24, total)
        parts.append(
            {
                "part_number": part_number,
                "title": f"第{part_number}部·失去的时间",
                "start_chapter": start,
                "end_chapter": end,
                "summary": "林岚沿未来来信追查旧车站的七分钟偏差，在与周衡互相试探、共同取证的过程中逐步确认实验钟正在改写城市记忆，并逼近顾沉试图挽回过去的真实目的。",
                "theme": "以求证建立信任并接受真实的失去",
                "key_events": ["未来来信预告停电", "地下机房发生时间回卷", "钟楼实验的代价被揭示"],
                "character_focus": {"林岚": "从独自承担秘密转向信任同伴，并选择守护真实时间线。"},
            }
        )
    return {"parts": parts}


def agent_chat_fixture(body: dict[str, Any]) -> dict[str, Any] | None:
    messages = body.get("messages")
    if not isinstance(messages, list):
        return None

    # The main Agent thread is durable, so a new turn carries the previous
    # turns' tool calls and child results as well.  Fixture decisions must be
    # scoped to the latest real user message; otherwise an old delegation can
    # make a fresh request look already completed.
    current_turn_start = next(
        (
            index
            for index in range(len(messages) - 1, -1, -1)
            if isinstance(messages[index], dict)
            and messages[index].get("role") == "user"
            and not str(messages[index].get("content", "")).startswith(
                "[Agent inbox:"
            )
        ),
        0,
    )
    current_messages = messages[current_turn_start:]

    capability = None
    for message in messages:
        if not isinstance(message, dict) or message.get("role") != "system":
            continue
        content = message.get("content")
        if not isinstance(content, str) or "你是 A4N 项目 Agent。" not in content:
            continue
        match = re.search(r"当前能力域(?:为|[:：])\s*([a-z0-9_]+)", content)
        if match:
            capability = match.group(1)
            break
    if capability is None:
        return None

    if any(
        str(message.get("content", "")).startswith("A4N_GENERAL ")
        for message in current_messages
        if message.get("role") == "user"
    ):
        from specialized_agent_fixture import general_fixture
        return general_fixture(body, messages)
    if capability in {"director", "canvas"} or any(
        str(message.get("content", "")).startswith("A4N_SPECIALIZED ")
        for message in current_messages if message.get("role") == "user"
    ):
        from specialized_agent_fixture import director_fixture
        return director_fixture(body, messages)
    if capability == "content_optimization":
        raise ValueError("content_optimization must enter the professional executor without an outer model call")
    if capability != "main":
        return None
    from specialized_agent_fixture import main_fixture
    return main_fixture(body, current_messages)


def message_text(content: Any) -> str:
    if isinstance(content, str):
        return content
    if not isinstance(content, list) or any(
        not isinstance(part, dict) or not isinstance(part.get("text"), str)
        for part in content
    ):
        raise ValueError("text fixture requires text content blocks")
    return "".join(part["text"] for part in content)


def cache_fixture_usage(protocol: str) -> dict[str, Any]:
    """Fixed billing counters test normalization, not a simulated cache engine."""
    if protocol == "anthropic":
        return {"input_tokens": 600, "cache_read_input_tokens": 9000, "cache_creation_input_tokens": 400, "output_tokens": 0}
    if protocol == "gemini":
        return {"promptTokenCount": 10000, "cachedContentTokenCount": 9000, "candidatesTokenCount": 100, "totalTokenCount": 10100}
    if protocol == "responses":
        return {"input_tokens": 10000, "output_tokens": 100, "total_tokens": 10100,
                "input_tokens_details": {"cached_tokens": 9000, "cache_write_tokens": 400}}
    return {"prompt_tokens": 10000, "completion_tokens": 100, "total_tokens": 10100,
            "prompt_tokens_details": {"cached_tokens": 9000, "cache_write_tokens": 400}}


def response_for(body: dict[str, Any]) -> str:
    messages = body.get("messages") or []
    combined = "\n".join(message_text(message.get("content", "")) for message in messages)
    if combined.strip() == "ping" or (len(messages) == 1 and "ping" in combined.lower()):
        return "pong"

    prompt_name = current_task_prompt_name(combined)
    if prompt_name == "reverse_blueprint":
        return json.dumps(reverse_blueprint_response(), ensure_ascii=False)
    if prompt_name == "reverse_outline":
        return json.dumps(reverse_outline_response(combined), ensure_ascii=False)
    if prompt_name == "reverse_part_outline":
        return json.dumps(reverse_part_outline_response(combined), ensure_ascii=False)
    if prompt_name == "galgame_branch_design":
        return json.dumps(galgame_branch_design_response(combined), ensure_ascii=False)
    if prompt_name == "galgame_script_adaptation":
        return json.dumps(galgame_script_adaptation_response(combined), ensure_ascii=False)
    if prompt_name == "galgame_character_staging":
        return json.dumps(galgame_character_staging_response(combined), ensure_ascii=False)
    if prompt_name == "galgame_visual_planning":
        return json.dumps(galgame_visual_planning_response(combined), ensure_ascii=False)

    manga_prompt_name = prompt_name
    if manga_prompt_name == "manga_character_binding":
        return json.dumps(manga_character_binding_response(combined), ensure_ascii=False)
    if manga_prompt_name == "manga_speech_selection":
        return json.dumps(manga_speech_selection_response(combined), ensure_ascii=False)
    if manga_prompt_name == "manga_visual_anchor_extraction":
        return json.dumps(manga_visual_anchor_response(combined), ensure_ascii=False)
    if manga_prompt_name == "manga_item_extraction":
        return json.dumps(manga_item_response(combined), ensure_ascii=False)
    if manga_prompt_name == "manga_chapter_comic_synthesis":
        return json.dumps(manga_synthesis_response(), ensure_ascii=False)
    if manga_prompt_name == "manga_page_planning":
        return json.dumps(manga_page_plan_response(combined), ensure_ascii=False)
    if manga_prompt_name == "manga_page_drawing_prompt":
        return json.dumps(manga_drawing_prompt_response(combined), ensure_ascii=False)
    if "漫画整页绘图提示词优化工程师" in combined:
        return json.dumps(manga_prompt_optimization_response(combined), ensure_ascii=False)

    if "人物三视图与人物拆解设定图 prompt" in combined:
        character_names = re.findall(r'"character_name"\s*:\s*"([^"]+)"', combined)
        character_name = character_names[-1] if character_names else "林岚"
        character_details = (
            "约30岁青年男性，黑色短发，深色工作夹克，提旧工具箱，神情克制警觉"
            if character_name == "周衡"
            else "28岁女性，黑色短发，深色风衣，随身携带旧怀表，神情冷静敏锐"
        )
        return (
            f"单张人物三视图与人物拆解设定图，{character_name}，{character_details}；"
            "横向浅灰设定图画布，左侧依次展示同一角色同一服装同一比例的正面、侧面、背面全身视图，"
            "中性正面站姿，完整展示头顶到鞋底；右侧人物拆解区包含发型结构、服饰层级拆解、随身道具细节、"
            "鞋履结构、深灰蓝色板与布料金属材质样片；都市时间悬疑电影概念设计，线条清晰，背景干净，"
            "不要普通单角度立绘，不要裁切手脚，不要复杂剧情背景。"
        )

    if "当前片段多格分镜板整理为可直接提交给视频生成模型" in combined:
        return json.dumps(video_prompt_response(), ensure_ascii=False)

    if "实际存在的边界帧参考图整理为稳定、具体" in combined:
        return json.dumps(video_prompt_response(), ensure_ascii=False)

    if "实际存在的起始画面约束整理为可直接复制" in combined:
        return json.dumps(video_prompt_response(), ensure_ascii=False)

    if "你是电影导演和视频生成提示词工程师" in combined:
        return json.dumps(chat_video_prompt_response(), ensure_ascii=False)

    if "你是视频分镜板提示词工程师" in combined:
        return json.dumps(video_storyboard_frame_response(), ensure_ascii=False)

    if "你是视频首尾帧提示词工程师" in combined:
        return json.dumps(video_static_frame_response(combined), ensure_ascii=False)

    if "你是视频首帧提示词工程师" in combined:
        return json.dumps(video_static_frame_response(combined), ensure_ascii=False)

    if "你是影视分段导演" in combined and "constraints.selected_duration_secs" in combined:
        return json.dumps(video_segment_plan_response(combined), ensure_ascii=False)

    if "你是导演分镜助手" in combined:
        return "从起点画面开始，镜头缓缓推进；人物沿通道走向门口并停步，光线随开门动作逐渐改变。遵循已给定的内部关键帧和场景，不推断下一镜头的图片。"

    if "chapter_shooting_design_unit" in combined or "ShootingSceneV1" in combined:
        return json.dumps(shooting_design_response(combined), ensure_ascii=False)

    if (
        "chapter_screenplay_unit_adaptation" in combined
        or "screenplay_unit_adaptation_v1" in combined
    ):
        return json.dumps(shooting_adaptation_response(combined), ensure_ascii=False)

    if (
        "chapter_screenplay_layout_unit" in combined
        or "screenplay_layout_unit_detail_v1" in combined
    ):
        return json.dumps(shooting_layout_response(combined), ensure_ascii=False)

    if "chapter_foreshadowing_events" in combined:
        return json.dumps(chapter_analysis_foreshadowing_response(combined), ensure_ascii=False)

    if "chapter_character_state" in combined:
        return json.dumps(chapter_analysis_character_state_response(), ensure_ascii=False)

    if "chapter_summary_synthesis" in combined:
        return json.dumps(chapter_analysis_summary_response(), ensure_ascii=False)

    if "chapter_fact_extraction" in combined:
        return json.dumps(chapter_analysis_facts_response(combined), ensure_ascii=False)

    if '"name": "object_discovery"' in combined:
        return json.dumps(object_discovery_response(combined), ensure_ascii=False)

    if '"name": "character_discovery"' in combined:
        return json.dumps(character_discovery_response(combined), ensure_ascii=False)

    if '"name": "scene_profile_candidates"' in combined:
        return json.dumps(scene_profile_candidates_response(combined), ensure_ascii=False)

    if '"name": "scene_occurrence_extraction"' in combined:
        return json.dumps(scene_occurrence_response(combined), ensure_ascii=False)

    if '"name": "scene_transition_extraction"' in combined:
        return json.dumps(scene_transition_response(combined), ensure_ascii=False)

    if '"name": "scene_entity_state_extraction"' in combined:
        return json.dumps(scene_entity_state_response(combined), ensure_ascii=False)

    if '"name": "character_sync_evidence_events"' in combined:
        return json.dumps(character_sync_evidence_events_response(combined), ensure_ascii=False)

    if '"name": "character_sync_attribute_changes"' in combined:
        return json.dumps(character_sync_attribute_changes_response(combined), ensure_ascii=False)

    if '"name": "character_sync_chapter_states"' in combined:
        return json.dumps(character_sync_chapter_states_response(combined), ensure_ascii=False)

    if '"name": "character_sync_behavior_records"' in combined:
        return json.dumps(character_sync_behaviors_response(combined), ensure_ascii=False)

    if '"name": "character_sync_deletion_candidates"' in combined:
        return json.dumps({"deletion_candidates": []}, ensure_ascii=False)

    if '"name": "implicit_classification"' in combined:
        return json.dumps(implicit_attribute_classification_response(combined), ensure_ascii=False)

    stage_match = re.search(r'"(?:stage_key|key)"\s*:\s*"(story_core|world|characters|relationships)"', combined)
    if stage_match:
        return json.dumps(blueprint_response(stage_match.group(1)), ensure_ascii=False)

    part_match = re.search(
        r'"generation_request"\s*:\s*\{[^{}]*"part_number"\s*:\s*(\d+)',
        combined,
        re.DOTALL,
    )
    if part_match and "character_focus" in combined and "key_events" in combined:
        return json.dumps(part_outline_response(int(part_match.group(1))), ensure_ascii=False)

    chapter_matches = re.findall(
        r'"wait_to_generate"\s*:\s*\{[^{}]*"start_chapter"\s*:\s*(\d+)',
        combined,
        re.DOTALL,
    )
    if chapter_matches and "word_count_target" in combined and "pacing_score" in combined:
        return json.dumps(
            {"chapters": [chapter_outline_response(int(chapter_matches[-1]))]},
            ensure_ascii=False,
        )

    if "## 正文字数要求" in combined and "不要输出 word_count" in combined:
        return chapter_body_response()

    if '"name": "chapter_scene_generation"' in combined:
        return chapter_body_response()

    if "content_optimization_chat_agent_loop" in combined:
        return json.dumps(optimization_chat_response(combined), ensure_ascii=False)

    if "content_optimization_agent_loop" in combined:
        return json.dumps(optimization_analysis_response(combined), ensure_ascii=False)

    if "content_optimization_patch" in combined and "old_text_must_match_once" in combined:
        return json.dumps(optimization_patch_response(), ensure_ascii=False)

    if "小说概念大师·文思" in combined or "conversation_state" in combined and "inspired_options" in combined:
        return json.dumps(inspiration_response(), ensure_ascii=False)

    if "蓝图优化" in combined and "full_synopsis" in combined:
        return json.dumps(
            {
                "style": "更紧凑的都市悬疑笔触，以钟表声和七分钟偏差作为反复意象",
                "full_synopsis": blueprint_response("story_core")["full_synopsis"] + "优化后，林岚更早主动选择相信周衡。",
            },
            ensure_ascii=False,
        )

    raise UnhandledPromptError(
        "No deterministic fixture matched prompt "
        + hashlib.sha256(combined.encode("utf-8")).hexdigest()[:16]
    )


class ProviderHandler(BaseHTTPRequestHandler):
    server: "ProviderServer"

    def log_message(self, _format: str, *_args: object) -> None:
        return

    def _json(self, status: int, value: Any) -> None:
        data = json.dumps(value, ensure_ascii=False).encode("utf-8")
        self.send_response(status)
        self.send_header("Content-Type", "application/json; charset=utf-8")
        self.send_header("Content-Length", str(len(data)))
        self.end_headers()
        self.wfile.write(data)

    def _bytes(self, status: int, content_type: str, data: bytes) -> None:
        self.send_response(status)
        self.send_header("Content-Type", content_type)
        self.send_header("Content-Length", str(len(data)))
        self.end_headers()
        self.wfile.write(data)

    def _has_bearer_token(self) -> bool:
        scheme, _, token = self.headers.get("Authorization", "").partition(" ")
        return scheme.lower() == "bearer" and bool(token.strip())

    def do_GET(self) -> None:
        normalized_path = self.path.split("?", 1)[0].rstrip("/")
        if normalized_path == "/v1beta/models":
            self._json(200, {"models": [{"name": "models/a4n-virtual-text", "displayName": "Virtual text"}]})
            return
        if normalized_path in {"/models", "/v1/models"}:
            self._json(
                200,
                {
                    "object": "list",
                    "data": [
                        {"id": "a4n-virtual-text", "object": "model"},
                        {"id": "a4n-virtual-embedding", "object": "model"},
                        {"id": "a4n-virtual-image", "object": "model"},
                        {"id": "a4n-virtual-video", "object": "model"},
                    ],
                },
            )
            return
        if normalized_path == "/health":
            self._json(200, {"status": "ok"})
            return
        if normalized_path == "/fixtures/video.mp4":
            self._bytes(200, "video/mp4", base64.b64decode(self.server.video_base64))
            return
        video_match = re.fullmatch(r"/(?:v1/)?videos/generations/([^/]+)", normalized_path)
        if video_match:
            self._get_video_generation(video_match.group(1))
            return
        if normalized_path.startswith("/task/"):
            task_id = normalized_path.removeprefix("/task/").strip()
            if not task_id:
                self._json(404, {"error": {"message": "task not found"}})
                return
            host, port = self.server.server_address
            self._json(
                200,
                {
                    "status": "completed",
                    "task_id": task_id,
                    "result": {"video_url": f"http://{host}:{port}/fixtures/video.mp4"},
                },
            )
            return
        self._json(404, {"error": {"message": "not found"}})

    def _get_video_generation(self, job_id: str) -> None:
        response = self.server.poll_video_job(job_id)
        if response is None:
            self._json(404, {"error": {"message": "video generation not found"}})
            return
        self._json(200, response)

    def do_DELETE(self) -> None:
        normalized_path = self.path.split("?", 1)[0].rstrip("/")
        if not normalized_path.startswith("/api/temp-media/"):
            self._json(404, {"error": {"message": "not found"}})
            return
        if not self._has_bearer_token():
            self._json(401, {"error": {"message": "missing bearer token"}})
            return
        length = int(self.headers.get("Content-Length", "0"))
        raw_body = self.rfile.read(length)
        try:
            body = json.loads(raw_body or b"{}")
        except json.JSONDecodeError as error:
            self._json(400, {"error": {"message": f"invalid json: {error}"}})
            return
        if not isinstance(body, dict):
            self._json(400, {"error": {"message": "request body must be a JSON object"}})
            return
        sha256 = body.get("sha256")
        if not isinstance(sha256, str) or not re.fullmatch(r"[0-9a-fA-F]{64}", sha256):
            self._json(400, {"error": {"message": "invalid sha256"}})
            return
        self.server.record(normalized_path, {"sha256": sha256.lower()})
        self.send_response(204)
        self.end_headers()

    def do_POST(self) -> None:
        length = int(self.headers.get("Content-Length", "0"))
        raw_body = self.rfile.read(length)
        normalized_path = self.path.split("?", 1)[0].rstrip("/")

        if normalized_path in {"/images/edits", "/v1/images/edits"}:
            content_type = self.headers.get("Content-Type", "")
            if "multipart/form-data" not in content_type:
                self._json(400, {"error": {"message": "image edit requires multipart/form-data"}})
                return
            required_fields = [b'name="model"', b'name="prompt"']
            has_image_field = b'name="image"' in raw_body or b'name="image[]"' in raw_body
            if any(field not in raw_body for field in required_fields) or not has_image_field:
                self._json(400, {"error": {"message": "image edit multipart is incomplete"}})
                return
            if not any(
                signature in raw_body
                for signature in (b"\x89PNG\r\n\x1a\n", b"\xff\xd8\xff", b"RIFF")
            ):
                self._json(400, {"error": {"message": "image edit has no image bytes"}})
                return
            self.server.record(
                self.path,
                {
                    "content_type": "multipart/form-data",
                    "body_bytes": len(raw_body),
                    "has_reference_image": True,
                },
            )
            self._json(200, {"created": 0, "data": [{"b64_json": self.server.image_base64}]})
            return

        try:
            body = json.loads(raw_body or b"{}")
        except json.JSONDecodeError as error:
            self._json(400, {"error": {"message": f"invalid json: {error}"}})
            return

        if normalized_path == "/api/temp-media":
            if not self._has_bearer_token():
                self._json(401, {"error": {"message": "missing bearer token"}})
                return
            try:
                media_bytes = base64.b64decode(body["data_base64"], validate=True)
                owner_user_id = int(body["owner_user_id"])
                project_id = int(body["project_id"])
                source_image_id = int(body["source_image_id"])
                expires_in_seconds = int(body["expires_in_seconds"])
                file_name = str(body["file_name"])
                mime_type = str(body["mime_type"])
            except (KeyError, TypeError, ValueError) as error:
                self._json(400, {"error": {"message": f"invalid temp media upload: {error}"}})
                return
            if min(owner_user_id, project_id, source_image_id, expires_in_seconds) <= 0:
                self._json(400, {"error": {"message": "temp media identity and ttl must be positive"}})
                return
            sha256 = hashlib.sha256(media_bytes).hexdigest()
            artifact_id = sha256[:32]
            expires_at = datetime.now(timezone.utc) + timedelta(
                seconds=min(expires_in_seconds, 86_340)
            )
            self.server.record(
                self.path,
                {
                    "owner_user_id": owner_user_id,
                    "project_id": project_id,
                    "source_image_id": source_image_id,
                    "source_media_kind": body.get("source_media_kind"),
                    "source_media_id": body.get("source_media_id"),
                    "file_name": file_name,
                    "mime_type": mime_type,
                    "byte_size": len(media_bytes),
                    "sha256": sha256,
                },
            )
            self._json(
                200,
                {
                    "artifact_id": artifact_id,
                    "url": f"https://media.example.test/{artifact_id}/{file_name}",
                    "expires_at": expires_at.isoformat().replace("+00:00", "Z"),
                    "owner_user_id": owner_user_id,
                    "project_id": project_id,
                    "source_image_id": source_image_id,
                    "source_media_kind": body.get("source_media_kind"),
                    "source_media_id": body.get("source_media_id"),
                    "content_type": mime_type,
                    "sha256": sha256,
                    "byte_size": len(media_bytes),
                },
            )
            return

        self.server.record(self.path, body)
        if self._native_llm(normalized_path, body):
            return
        if normalized_path in {"/embeddings", "/v1/embeddings"}:
            inputs = body.get("input")
            if isinstance(inputs, str):
                inputs = [inputs]
            if not isinstance(inputs, list) or not inputs or not all(
                isinstance(value, str) for value in inputs
            ):
                self._json(400, {"error": {"message": "embedding input must be non-empty text/list"}})
                return
            self._json(
                200,
                {
                    "object": "list",
                    "data": [
                        {
                            "object": "embedding",
                            "index": index,
                            "embedding": embedding_for(value, self.server.embedding_dimension),
                        }
                        for index, value in enumerate(inputs)
                    ],
                    "model": body.get("model", "a4n-virtual-embedding"),
                    "usage": {"prompt_tokens": len(inputs), "total_tokens": len(inputs)},
                },
            )
            return
        if normalized_path in {"/images/generations", "/v1/images/generations"}:
            self._json(200, {"created": 0, "data": [{"b64_json": self.server.image_base64}]})
            return
        if normalized_path in {"/videos/generations", "/v1/videos/generations"}:
            idempotency_key = self.headers.get("Idempotency-Key", "").strip() or None
            try:
                response = self.server.create_video_job(body, idempotency_key)
            except VideoIdempotencyConflict as error:
                self._json(409, {"error": {"message": str(error)}})
                return
            self._json(200, response)
            return
        if normalized_path == "/task/submit":
            parameters = body.get("parameters")
            media_urls = []
            if isinstance(parameters, dict):
                for field in ("image_files", "video_files", "audio_files"):
                    value = parameters.get(field, [])
                    if isinstance(value, list):
                        media_urls.extend(value)
            if (
                not isinstance(body.get("model_id"), str)
                or not isinstance(parameters, dict)
                or parameters.get("functionMode") != "omni_reference"
                or not media_urls
                or not all(isinstance(url, str) and url.startswith("https://") for url in media_urls)
            ):
                self._json(400, {"error": {"message": "invalid Seedance omni_reference request"}})
                return
            self._json(200, {"status": "submitted", "task_id": "a4n-e2e-seedance-job"})
            return
        if normalized_path not in {"/chat/completions", "/v1/chat/completions"}:
            self._json(404, {"error": {"message": "not found"}})
            return

        try:
            lifecycle_fixture = str(body.get("model", "")).removeprefix("a4n-llm-lifecycle-")
            if lifecycle_fixture in {"ok", "length", "missing", "slow"}:
                agent_fixture = {"content": "partial answer " * 200 if lifecycle_fixture == "slow" else "LLM lifecycle fixture output"}
            elif body.get("a4n_test_cache_usage") is True:
                agent_fixture = {"content": "雨夜里，角色谨慎地推开仓库门。"}
            else:
                agent_fixture = agent_chat_fixture(body)
        except (ValueError, KeyError, TypeError) as error:
            self._json(422, {"error": {"message": str(error), "type": "invalid_agent_fixture"}})
            return
        if agent_fixture is None:
            try:
                content = response_for(body)
            except (UnhandledPromptError, ValueError) as error:
                self._json(422, {"error": {"message": str(error), "type": "unhandled_prompt"}})
                return
            tool_calls = []
        else:
            content = str(agent_fixture.get("content", ""))
            tool_calls = agent_fixture.get("tool_calls") or ([agent_fixture["tool_call"]] if agent_fixture.get("tool_call") else [])
        if body.get("stream") is True:
            self.send_response(200)
            self.send_header("Content-Type", "text/event-stream; charset=utf-8")
            self.send_header("Cache-Control", "no-cache")
            self.end_headers()
            if tool_calls:
                events = []
                for tool_index, tool_call in enumerate(tool_calls):
                    arguments = json.dumps(tool_call["arguments"], ensure_ascii=False, separators=(",", ":"))
                    split_at = max(1, len(arguments) // 2)
                    deltas = [
                        {"id": tool_call["id"], "type": "function"},
                        {"function": {"name": tool_call["name"]}},
                        *({"function": {"arguments": fragment}} for fragment in (arguments[:split_at], arguments[split_at:]) if fragment),
                    ]
                    events.extend({"choices": [{"index": 0, "delta": {"tool_calls": [{"index": tool_index, **delta}]}, "finish_reason": None}]} for delta in deltas)
                events.append(
                    {
                        "choices": [
                            {"index": 0, "delta": {}, "finish_reason": "tool_calls"}
                        ],
                        "usage": {
                            "prompt_tokens": 10,
                            "completion_tokens": 20,
                            "total_tokens": 30,
                        },
                    }
                )
                for event in events:
                    self.wfile.write(
                        ("data: " + json.dumps(event, ensure_ascii=False) + "\n\n").encode(
                            "utf-8"
                        )
                    )
                self.wfile.write(b"data: [DONE]\n\n")
                self.wfile.flush()
                return
            chunk_size = 32 if lifecycle_fixture == "slow" else 1024
            for index in range(0, len(content), chunk_size):
                event = {
                    "choices": [
                        {
                            "index": 0,
                            "delta": {"content": content[index : index + chunk_size]},
                            "finish_reason": None,
                        }
                    ]
                }
                try:
                    self.wfile.write(("data: " + json.dumps(event, ensure_ascii=False) + "\n\n").encode("utf-8"))
                    if lifecycle_fixture == "slow":
                        self.wfile.flush()
                        time.sleep(0.05)
                except (BrokenPipeError, ConnectionResetError):
                    # Cancellation fixtures deliberately close an active response.
                    return
            final = {
                "choices": [{"index": 0, "delta": {}, "finish_reason": {"length": "length", "missing": None}.get(lifecycle_fixture, "stop")}],
                "usage": {"prompt_tokens": 10, "completion_tokens": 20, "total_tokens": 30},
            }
            if body.get("a4n_test_cache_usage") is True:
                final["usage"] = cache_fixture_usage("chat")
            self.wfile.write(("data: " + json.dumps(final) + "\n\n").encode("utf-8"))
            self.wfile.write(b"data: [DONE]\n\n")
            self.wfile.flush()
            return

        if tool_calls:
            self._json(
                200,
                {
                    "id": "chatcmpl-a4n-e2e",
                    "object": "chat.completion",
                    "choices": [
                        {
                            "index": 0,
                            "message": {
                                "role": "assistant",
                                "content": "",
                                "tool_calls": [
                                    {
                                        "id": tool_call["id"],
                                        "type": "function",
                                        "function": {
                                            "name": tool_call["name"],
                                            "arguments": json.dumps(
                                                tool_call["arguments"],
                                                ensure_ascii=False,
                                                separators=(",", ":"),
                                            ),
                                        },
                                    } for tool_call in tool_calls
                                ],
                            },
                            "finish_reason": "tool_calls",
                        }
                    ],
                    "usage": {
                        "prompt_tokens": 10,
                        "completion_tokens": 20,
                        "total_tokens": 30,
                    },
                },
            )
            return

        self._json(
            200,
            {
                "id": "chatcmpl-a4n-e2e",
                "object": "chat.completion",
                "choices": [
                    {
                        "index": 0,
                        "message": {"role": "assistant", "content": content},
                        "finish_reason": "stop",
                    }
                ],
                "usage": {"prompt_tokens": 10, "completion_tokens": 20, "total_tokens": 30},
            },
        )


    def _native_llm(self, path: str, body: dict[str, Any]) -> bool:
        """Wire fixtures for native text protocols; reuse the prompt fixture owner."""
        if path not in {"/v1/responses", "/v1/messages", "/v1beta/models/a4n-virtual-text:streamGenerateContent"}:
            return False
        try:
            if path == "/v1/responses":
                if not self._has_bearer_token() or body.get("stream") is not True or not isinstance(body.get("input"), list):
                    raise ValueError("Responses requires bearer auth, stream and input[]")
                messages = [{"role": item["role"], "content": item["content"]} for item in body["input"] if "role" in item]
            elif path == "/v1/messages":
                if not self.headers.get("x-api-key") or not self.headers.get("anthropic-version") or body.get("stream") is not True:
                    raise ValueError("Messages requires native auth/version and stream")
                messages = [{"role": "system", "content": message_text(body.get("system", []))}]
                messages += [{"role": item["role"], "content": message_text(item["content"])} for item in body["messages"]]
            else:
                if not self.headers.get("x-goog-api-key") or "alt=sse" not in self.path:
                    raise ValueError("Gemini requires native auth and alt=sse")
                messages = [{"role": "system", "content": "\n".join(part["text"] for part in body.get("systemInstruction", {}).get("parts", []))}]
                messages += [{"role": "assistant" if item["role"] == "model" else "user", "content": "\n".join(part["text"] for part in item["parts"])} for item in body["contents"]]
            content = "雨夜里，角色谨慎地推开仓库门。" if body.get("a4n_test_cache_usage") is True else response_for({"messages": messages})
        except (KeyError, TypeError, ValueError, UnhandledPromptError) as error:
            self._json(422, {"error": {"message": str(error)}})
            return True
        if path == "/v1/responses":
            events = [
                {"type": "response.output_text.delta", "delta": content},
                {"type": "response.completed", "response": {"status": "completed", "output": [
                    {"type": "message", "role": "assistant", "content": [{"type": "output_text", "text": content}]}
                ], "usage": {"input_tokens": 2, "output_tokens": 1, "total_tokens": 3}}},
            ]
        elif path == "/v1/messages":
            events = [
                {"type": "message_start", "message": {"usage": {"input_tokens": 2, "output_tokens": 0}}},
                {"type": "content_block_start", "index": 0, "content_block": {"type": "text", "text": ""}},
                {"type": "content_block_delta", "index": 0, "delta": {"type": "text_delta", "text": content}},
                {"type": "content_block_stop", "index": 0},
                {"type": "message_delta", "delta": {"stop_reason": "end_turn"}, "usage": {"output_tokens": 1}},
                {"type": "message_stop"},
            ]
        else:
            events = [{"candidates": [{"content": {"role": "model", "parts": [{"text": content}]}, "finishReason": "STOP"}],
                       "usageMetadata": {"promptTokenCount": 2, "candidatesTokenCount": 1, "totalTokenCount": 3}}]
        if body.get("a4n_test_cache_usage") is True:
            if path == "/v1/responses":
                events[-1]["response"]["usage"] = cache_fixture_usage("responses")
            elif path == "/v1/messages":
                events[0]["message"]["usage"] = cache_fixture_usage("anthropic")
                events[-2]["usage"]["output_tokens"] = 100
            else:
                events[0]["usageMetadata"] = cache_fixture_usage("gemini")
        data = "".join(f"data: {json.dumps(event, ensure_ascii=False)}\n\n" for event in events).encode("utf-8")
        self._bytes(200, "text/event-stream", data)
        return True


class ProviderServer(ThreadingHTTPServer):
    def __init__(
        self,
        address: tuple[str, int],
        log_path: Path,
        image_base64: str,
        video_base64: str,
        embedding_dimension: int,
    ):
        super().__init__(address, ProviderHandler)
        self.log_path = log_path
        self.image_base64 = image_base64
        self.video_base64 = video_base64
        self.embedding_dimension = embedding_dimension
        self.log_lock = threading.Lock()
        self.request_index = 0
        self.video_lock = threading.Lock()
        self.video_jobs: dict[str, dict[str, Any]] = {}
        self.video_idempotency: dict[str, tuple[str, str]] = {}
        self.video_job_index = 0

    def create_video_job(
        self, body: dict[str, Any], idempotency_key: str | None
    ) -> dict[str, Any]:
        fingerprint = json.dumps(
            body, ensure_ascii=False, sort_keys=True, separators=(",", ":")
        )
        with self.video_lock:
            if idempotency_key is not None:
                existing = self.video_idempotency.get(idempotency_key)
                if existing is not None:
                    job_id, existing_fingerprint = existing
                    if existing_fingerprint != fingerprint:
                        raise VideoIdempotencyConflict(
                            "Idempotency-Key 已用于不同的视频请求"
                        )
                    return self._video_job_response_locked(job_id)

            self.video_job_index += 1
            job_id = (
                "a4n-e2e-video-job"
                if self.video_job_index == 1
                else f"a4n-e2e-video-job-{self.video_job_index}"
            )
            self.video_jobs[job_id] = {
                "body": body,
                "polls": 0,
                "status": "pending",
            }
            if idempotency_key is not None:
                self.video_idempotency[idempotency_key] = (job_id, fingerprint)
            return self._video_job_response_locked(job_id)

    def poll_video_job(self, job_id: str) -> dict[str, Any] | None:
        with self.video_lock:
            if job_id not in self.video_jobs:
                return None
            job = self.video_jobs[job_id]
            job["polls"] += 1
            job["status"] = (
                "pending"
                if job["polls"] == 1
                else "running"
                if job["polls"] == 2
                else "completed"
            )
            return self._video_job_response_locked(job_id)

    def _video_job_response_locked(self, job_id: str) -> dict[str, Any]:
        job = self.video_jobs[job_id]
        response: dict[str, Any] = {
            "id": job_id,
            "status": job["status"],
        }
        if job["status"] == "completed":
            response["data"] = [
                {"b64_json": self.video_base64, "format": "mp4"}
            ]
        return response

    def record(self, path: str, body: dict[str, Any]) -> None:
        with self.log_lock:
            self.request_index += 1
            entry = {"index": self.request_index, "path": path, "body": body}
            with self.log_path.open("a", encoding="utf-8") as log:
                log.write(json.dumps(entry, ensure_ascii=False) + "\n")


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--host", default="127.0.0.1")
    parser.add_argument("--port", type=int, default=18765)
    parser.add_argument("--log", type=Path, required=True)
    parser.add_argument("--image-file", type=Path)
    parser.add_argument("--video-file", type=Path)
    parser.add_argument("--embedding-dimension", type=int, default=8)
    args = parser.parse_args()
    if args.embedding_dimension <= 0:
        parser.error("--embedding-dimension must be positive")
    args.log.parent.mkdir(parents=True, exist_ok=True)
    image_base64 = (
        base64.b64encode(args.image_file.read_bytes()).decode("ascii")
        if args.image_file
        else PNG_1X1_BASE64
    )
    video_base64 = (
        base64.b64encode(args.video_file.read_bytes()).decode("ascii")
        if args.video_file
        else MP4_1S_BASE64
    )
    ProviderServer(
        (args.host, args.port),
        args.log,
        image_base64,
        video_base64,
        args.embedding_dimension,
    ).serve_forever()


if __name__ == "__main__":
    main()
