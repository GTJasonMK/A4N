#!/usr/bin/env python3
"""Deterministic loopback-only OpenAI-compatible provider for CLI E2E tests."""

from __future__ import annotations

import argparse
import base64
import hashlib
import json
import re
import threading
from datetime import datetime, timedelta, timezone
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from typing import Any


PNG_1X1_BASE64 = (
    "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNk+A8AAQUBAScY42YAAAAASUVORK5CYII="
)


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
    pairs = re.findall(r'"id"\s*:\s*(\d+)\s*,\s*"quote"\s*:\s*"([^"]+)"', combined)
    for anchor_id, quote in pairs:
        if phrase in quote:
            return int(anchor_id)
    raise ValueError(f"prompt contains no text anchor for {phrase}")


def named_profile_id(combined: str, name: str, field: str) -> int:
    match = re.search(
        rf'"name"\s*:\s*"{re.escape(name)}"\s*,\s*"{field}"\s*:\s*(\d+)',
        combined,
    )
    if not match:
        raise ValueError(f"prompt contains no {field} for {name}")
    return int(match.group(1))


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
                "evidence_anchor_ids": [text_anchor_id(combined, "怀表继续逆行")],
            },
            {
                "name": "齿轮暗记",
                "aliases": ["齿轮暗号"],
                "object_type": "token",
                "importance": "secondary",
                "visual_description": "划在墙面、形似齿轮的识别暗记",
                "material_and_state": "新近补画在封闭通道入口墙面",
                "story_usage": "记录位置并作为后续追查线索",
                "visual_usage": "作为封闭通道与地下机房入口的空间标记",
                "generation_usage": "墙面保留清晰但隐蔽的齿轮形暗记",
                "location_context": "旧车站封闭通道入口",
                "evidence_anchor_ids": [text_anchor_id(combined, "补记了齿轮暗号")],
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
                "explicit_attributes": {"位置": "地下机房入口", "持有物": "旧怀表"},
                "implicit_attributes": {"目标": "调查时间异常", "态度": "警觉"},
                "social_attributes": {"周衡": "暂不信任"},
                "evidence_anchor_id": text_anchor_id(combined, "林岚重新核对"),
            },
            {
                "name": "周衡",
                "aliases": [],
                "importance": "secondary",
                "role": "调查对象",
                "explicit_attributes": {"位置": "旧车站封闭通道", "异常": "影子未随画面回退"},
                "implicit_attributes": {"动机": "尚未明确"},
                "social_attributes": {"林岚": "受到怀疑"},
                "evidence_anchor_id": text_anchor_id(combined, "周衡的影子"),
            },
        ]
    }


def scene_profile_candidates_response(combined: str) -> dict[str, Any]:
    return {
        "scenes": [
            {
                "scene_ref": "station_passage",
                "scene_profile_id": None,
                "name": "旧车站封闭通道",
                "aliases": ["封闭通道"],
                "scene_type": "route",
                "importance": "primary",
                "parent_scene_id": None,
                "base_description": "连接旧车站站厅与地下区域的封闭通道",
                "visual_description": "墙面留有齿轮暗记，广告屏冷光映入入口",
                "spatial_layout": "入口连着站厅，通道向下延伸至地下机房",
                "atmosphere": "停电后的昏暗冷光，机械钟鸣造成紧张感",
                "entry_points": ["站厅入口", "地下机房方向"],
                "rules": ["电子画面会发生七秒回退"],
                "evidence_anchor_ids": [text_anchor_id(combined, "封闭通道入口")],
            },
            {
                "scene_ref": "underground_machine_room",
                "scene_profile_id": None,
                "name": "地下机房",
                "aliases": [],
                "scene_type": "room",
                "importance": "primary",
                "parent_scene_id": None,
                "base_description": "旧车站地下用于容纳时间装置的机房",
                "visual_description": "由封闭通道进入的地下机械空间",
                "spatial_layout": "入口位于封闭通道尽头，内部空间尚未完全揭示",
                "atmosphere": "未知、压迫，时间异常来源仍未显现",
                "entry_points": ["封闭通道"],
                "rules": [],
                "evidence_anchor_ids": [text_anchor_id(combined, "进入地下机房")],
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
                "occurrence_ref": "occ_station_passage_1",
                "scene_ref": "station_passage",
                "timeline_position": "暴雨夜午夜前",
                "summary": "林岚在封闭通道入口留下齿轮暗记，并观察到周衡影子静止的异常。",
                "mood": "警觉紧张",
                "weather_or_lighting": "停电后的广告屏冷光",
                "source_anchor_ids": [
                    text_anchor_id(combined, "补记了齿轮暗号"),
                    text_anchor_id(combined, "周衡的影子"),
                ],
                "characters": [
                    {"profile_id": lin_lan, "role": "留下暗记并判断异常"},
                    {"profile_id": zhou_heng, "role": "影子表现出时间异常"},
                ],
                "objects": [
                    {"object_profile_id": watch, "role": "由林岚随身携带"},
                    {"object_profile_id": gear_mark, "role": "标记通道入口"},
                ],
                "key_events": ["林岚确认周衡身上存在异常"],
                "entry_state": {},
                "exit_state": {},
            },
            {
                "occurrence_ref": "occ_machine_room_1",
                "scene_ref": "underground_machine_room",
                "timeline_position": "第二声钟鸣后",
                "summary": "林岚进入地下机房，以行动继续验证周衡是否可信。",
                "mood": "坚定而压迫",
                "weather_or_lighting": None,
                "source_anchor_ids": [
                    text_anchor_id(combined, "进入地下机房"),
                    text_anchor_id(combined, "检验周衡是否可信"),
                ],
                "characters": [{"profile_id": lin_lan, "role": "进入机房继续调查"}],
                "objects": [{"object_profile_id": watch, "role": "继续作为时间异常线索"}],
                "key_events": ["林岚进入地下机房"],
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
                "to_scene_ref": "station_passage",
                "occurrence_ref": "occ_station_passage_1",
                "movement_type": "enter",
                "route_description": "林岚来到旧车站封闭通道入口",
                "time_cost": None,
                "distance_hint": None,
                "participants": [lin_lan],
                "object_refs": [watch],
                "source_anchor_ids": [text_anchor_id(combined, "封闭通道入口")],
            },
            {
                "from_scene_ref": "station_passage",
                "to_scene_ref": "underground_machine_room",
                "occurrence_ref": "occ_machine_room_1",
                "movement_type": "travel",
                "route_description": "林岚沿封闭通道进入地下机房",
                "time_cost": None,
                "distance_hint": "通道尽头",
                "participants": [lin_lan],
                "object_refs": [watch],
                "source_anchor_ids": [text_anchor_id(combined, "进入地下机房")],
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
                "scene_ref": "station_passage",
                "occurrence_ref": "occ_station_passage_1",
                "entity_type": "character",
                "entity_id": lin_lan,
                "state_key": "possession",
                "state_before": {},
                "state_after": {"object_profile_id": watch},
                "position_description": "封闭通道入口",
                "action_description": "林岚随身持有逆行的旧怀表",
                "source_anchor_ids": [text_anchor_id(combined, "怀表继续逆行")],
            },
            {
                "scene_ref": "station_passage",
                "occurrence_ref": "occ_station_passage_1",
                "entity_type": "object",
                "entity_id": gear_mark,
                "state_key": "position",
                "state_before": {},
                "state_after": {"location": "封闭通道入口墙面"},
                "position_description": "封闭通道入口墙面",
                "action_description": "齿轮暗记被补画在墙面",
                "source_anchor_ids": [text_anchor_id(combined, "补记了齿轮暗号")],
            },
            {
                "scene_ref": "station_passage",
                "occurrence_ref": "occ_station_passage_1",
                "entity_type": "character",
                "entity_id": zhou_heng,
                "state_key": "condition",
                "state_before": {},
                "state_after": {"shadow": "未随广告画面回退"},
                "position_description": "封闭通道附近",
                "action_description": "周衡的影子停在原地",
                "source_anchor_ids": [text_anchor_id(combined, "周衡的影子")],
            },
            {
                "scene_ref": "underground_machine_room",
                "occurrence_ref": "occ_machine_room_1",
                "entity_type": "character",
                "entity_id": lin_lan,
                "state_key": "position",
                "state_before": {"scene_ref": "station_passage"},
                "state_after": {"scene_ref": "underground_machine_room"},
                "position_description": "地下机房内",
                "action_description": "林岚进入地下机房继续调查",
                "source_anchor_ids": [text_anchor_id(combined, "进入地下机房")],
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
                    "summary": "旧怀表继续逆行，林岚确认七分钟时间异常真实发生。",
                    "evidence_anchor_id": text_anchor_id(combined, "怀表继续逆行"),
                    "related_characters": [],
                },
                {
                    "chapter_number": 1,
                    "event_type": "decision",
                    "summary": "林岚进入地下机房，以行动检验周衡是否可信。",
                    "evidence_anchor_id": text_anchor_id(combined, "检验周衡是否可信"),
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
                    "evidence_anchor_id": text_anchor_id(combined, "周衡的影子"),
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
                    "new_value": "地下机房入口",
                    "change_description": "林岚已从封闭通道进入地下机房。",
                    "event_cause": "主动继续调查时间异常",
                    "evidence_anchor_id": text_anchor_id(combined, "进入地下机房"),
                },
                {
                    "chapter_number": 1,
                    "category": "explicit",
                    "key": "current_goal",
                    "operation": "add",
                    "new_value": "调查时间异常并检验周衡是否可信",
                    "change_description": "本章明确形成新的即时调查目标。",
                    "event_cause": "周衡的影子表现出异常",
                    "evidence_anchor_id": text_anchor_id(combined, "检验周衡是否可信"),
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
                    "evidence_anchor_id": text_anchor_id(combined, "她没有把信交给周衡"),
                },
                {
                    "chapter_number": 1,
                    "category": "explicit",
                    "key": "time_anomaly",
                    "operation": "add",
                    "new_value": "影子未随广告画面回退",
                    "change_description": "周衡出现可观察的时间异常特征。",
                    "event_cause": "第二声钟鸣后广告画面倒退",
                    "evidence_anchor_id": text_anchor_id(combined, "周衡的影子"),
                },
            ]
        }
    return {"attribute_changes": []}


def character_sync_chapter_states_response(combined: str) -> dict[str, Any]:
    name = character_sync_profile_name(combined)
    if name == "林岚":
        state = {
            "chapter_number": 1,
            "state_summary": "确认时间异常后进入地下机房，继续调查并保持对周衡的警惕。",
            "current_location": "地下机房入口",
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
        return {"chapter_states": []}
    return {"chapter_states": [state]}


def character_sync_behaviors_response(combined: str) -> dict[str, Any]:
    name = character_sync_profile_name(combined)
    if name == "林岚":
        return {
            "behaviors": [
                {
                    "chapter_number": 1,
                    "behavior_type": "investigation",
                    "behavior_description": "留下齿轮暗记后进入地下机房继续调查。",
                    "original_text_anchor_id": text_anchor_id(combined, "进入地下机房"),
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
                    "original_text_anchor_id": text_anchor_id(combined, "周衡的影子"),
                    "behavior_tags": ["时间异常", "身份疑点"],
                    "related_characters": ["林岚"],
                    "impact_analysis": "加深林岚对周衡身份和动机的怀疑。",
                }
            ]
        }
    return {"behaviors": []}


def implicit_attribute_classification_response(combined: str) -> dict[str, Any]:
    if character_sync_profile_name(combined) == "林岚":
        return {
            "classifications": {"personality": "conform"},
            "reasoning": "她留下暗记后主动进入地下机房，符合冷静求证并持续追查异常的性格。",
            "suggested_new_attributes": [],
        }
    return {
        "classifications": {},
        "reasoning": "影子异常只能证明时间现象，不能据此判断当前性格属性。",
        "suggested_new_attributes": [],
    }


def chapter_analysis_facts_response(combined: str) -> dict[str, Any]:
    lin_lan = source_span_id(combined, "林岚重新核对")
    old_station = source_span_id(combined, "旧车站停电")
    watch = source_span_id(combined, "怀表继续逆行")
    zhou_heng = source_span_id(combined, "周衡的影子")
    gear_mark = source_span_id(combined, "齿轮暗号")
    decision = source_span_id(combined, "进入地下机房")
    return {
        "characters": [
            {"name": "林岚", "source_span_ids": [lin_lan]},
            {"name": "周衡", "source_span_ids": [zhou_heng]},
        ],
        "locations": [
            {"name": "旧车站", "source_span_ids": [old_station]},
            {"name": "地下机房", "source_span_ids": [decision]},
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
                "event_ref": "event-enter-machine-room",
                "type": "decision",
                "description": "林岚留下齿轮暗记后进入地下机房，以行动检验周衡是否可信。",
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
            "compressed": "林岚确认旧车站发生七分钟时间倒退，并在留下齿轮暗记后进入地下机房调查周衡。",
            "one_line": "林岚循着时间异常进入地下机房。",
            "keywords": ["旧车站", "七分钟", "怀表", "周衡", "地下机房"],
        },
    }


def chapter_analysis_character_state_response() -> dict[str, Any]:
    return {
        "character_states": {
            "林岚": {
                "location": "地下机房入口",
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
            "tensions": ["林岚尚未确认周衡是否可信", "地下机房中的时间装置仍未揭示"],
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


def shooting_span_ids(combined: str) -> list[str]:
    ids = [span_id for span_id, _ in source_span_pairs(combined)]
    if not ids:
        ids = re.findall(r"ch\d+:s\d+:f\d+", combined)
    ids = list(dict.fromkeys(ids))
    if not ids:
        raise ValueError("shooting script prompt contains no source spans")
    return ids


def shooting_directing_statement() -> dict[str, str]:
    return {
        "central_dramatic_question": "林岚能否确认时间倒退并判断周衡是否可信？",
        "protagonist_throughline": "林岚从核对异常证据走向主动进入地下机房求证。",
        "audience_experience": "先确认七分钟倒退，再因周衡静止的影子产生新的怀疑。",
        "viewpoint_strategy": "限制在林岚能够观察和判断的信息范围内。",
        "visual_strategy": "以逆行怀表、回退广告屏和静止影子构成递进视觉证据。",
        "spatial_strategy": "从封闭通道入口沿明确轴线推进到地下机房。",
        "sound_strategy": "用两次钟鸣、停电后的环境底噪和短暂停顿制造压力。",
        "rhythm_strategy": "核对、异常显现、留下暗记、进入机房，逐步加速。",
    }


def shooting_skeleton_response(combined: str) -> dict[str, Any]:
    return {
        "schema_version": "chapter_screenplay_skeleton_v1",
        "directing_statement": shooting_directing_statement(),
        "units": [
            {
                "scene_identity": "station-passage-to-machine-room",
                "sequence_index": 1,
                "narrative_unit_index": 1,
                "timeline_position": 1,
                "intercut_group_id": None,
                "editing_mode": "continuity",
                "source_span_ids": shooting_span_ids(combined),
            }
        ],
    }


def shooting_layout_response() -> dict[str, Any]:
    return {
        "schema_version": "screenplay_layout_unit_detail_v1",
        "narrative_unit_index": 1,
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
    evidence = source_span_id(combined, "进入地下机房", len(shooting_span_ids(combined)) - 1)
    return {
        "schema_version": "screenplay_unit_adaptation_v1",
        "narrative_unit_index": 1,
        "performance_facts": [
            {
                "fact_index": 1,
                "source_span_ids": [evidence],
                "actor": "林岚",
                "objective": "用行动验证周衡是否可信。",
                "obstacle": "时间异常与周衡的反常影子都缺少解释。",
                "visible_action": "在墙上划下齿轮暗记后走入地下机房。",
                "reaction": "短暂停顿后收紧怀表。",
                "dialogue_intent": None,
                "entry_state": "警觉地观察周衡。",
                "exit_state": "坚定进入地下机房继续调查。",
                "required": True,
            }
        ],
    }


def shooting_design_response(combined: str) -> dict[str, Any]:
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


def video_segment_plan_response() -> dict[str, Any]:
    return {
        "segments": [
            {
                "source_shot_id": "unit_1_shot_1",
                "covered_beat_ids": ["unit_1_shot_1_beat_1"],
                "title": "暗记指向地下机房",
                "scene_summary": "林岚把对周衡的警觉转成可见行动：刻下齿轮暗记、收紧怀表，并独自走向地下机房。",
                "blocking_summary": "林岚从封闭通道墙面左侧起步，刻下居中的暗记后向画面右侧纵深的机房入口移动，周衡始终留在后方。",
                "spatial_anchors": [
                    {
                        "anchor": "封闭通道墙面",
                        "usage": "约束林岚、齿轮暗记和地下机房入口的空间关系",
                    }
                ],
                "emotion_action_goals": [
                    {
                        "character": "林岚",
                        "emotion": "警觉转为坚定",
                        "visible_action": "刻下齿轮暗记，收紧怀表，回望周衡后迈入地下机房",
                    }
                ],
                "video_intent": {
                    "hook": "齿轮暗记把时间异常与地下机房连成一条可见线索",
                    "audience_effect": "让观众感到林岚正在主动踏入更危险的真相",
                    "rhythm": "观察停顿后连续推进，结尾保留进入黑暗的悬念",
                    "visual_priority": "刻下暗记、收紧怀表和跨入机房入口的连续动作",
                },
                "emotional_arc": {
                    "start": "警觉地观察周衡与通道异常",
                    "turn": "刻下暗记后决定不再等待解释",
                    "end": "坚定地独自进入地下机房",
                    "function": "把怀疑转成行动并推进到下一空间",
                },
                "performance_plan": {
                    "movement_path": "从画面左侧封闭通道墙面起步，刻下暗记后转向画面右侧纵深的地下机房入口，并在门槛处跨入黑暗。",
                    "body_performance": "起初肩颈收紧、重心靠后，决定行动后收起怀表、重心前移，以克制而稳定的步幅进入机房。",
                    "gaze_and_hands": "视线先落在墙面暗记，再回望后方周衡；右手刻下暗记并合拢怀表，随后贴近身体保护道具。",
                    "pause_or_reaction": "刻完暗记后停半拍回望周衡，确认无人阻拦才迈向机房，结尾在黑暗吞没背影前留出短暂余韵。",
                    "camera_relationship": "平视中景保持墙面、林岚和机房入口同框，前半段轻微固定观察，后半段沿画面右侧纵深跟拍。",
                    "time_blocks": [
                        {
                            "start_sec": 0,
                            "end_sec": 3,
                            "camera": "平视中景固定林岚与墙面，机房入口保留在画面右侧纵深。",
                            "subject_action": "林岚刻下齿轮暗记，收紧怀表并停半拍回望周衡。",
                            "continuity_target": "暗记居中，怀表保持在林岚右手，机房入口仍在画面右侧。",
                        },
                        {
                            "start_sec": 3,
                            "end_sec": 6,
                            "camera": "中景沿右侧纵深缓慢跟拍至地下机房入口。",
                            "subject_action": "林岚重心前移，稳定迈步并跨入地下机房。",
                            "continuity_target": "林岚背影进入黑暗，周衡留在通道后方，空间方向不变。",
                        },
                    ],
                    "action_script": [
                        {
                            "character": "林岚",
                            "body": "肩颈由紧绷转为稳定，迈步时重心前移。",
                            "movement": "从画面左侧墙面转向右侧纵深，最终跨入地下机房入口。",
                            "hands": "右手刻下暗记并合拢怀表，随后让怀表贴近身体。",
                            "gaze": "从墙面暗记移向周衡，再锁定地下机房入口。",
                            "timing": "0至3秒刻记并回望，3至6秒转身进入机房。",
                        }
                    ],
                },
                "material_coverage": {
                    "required_entities": ["林岚", "旧怀表", "封闭通道", "地下机房入口"],
                    "missing_or_weak_references": [],
                    "risk_note": "关键对象已有档案或文字约束足够",
                },
            }
        ]
    }


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


def galgame_branch_design_response(combined: str) -> dict[str, Any]:
    task_input = json_task_payload(combined)["input"]
    scene = task_input["previous_step"]["shooting_script_projection"]["scenes"][0]
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
    previous = task_input["previous_steps"]
    scene = previous["shooting_script_projection"]["scenes"][0]
    beat_ref = galgame_beat_ref(scene)
    source_spans = task_input["chapters"][0]["source_spans"]
    first_span = galgame_source_span_id(source_spans, "雨", 0)
    zhou_span = galgame_source_span_id(source_spans, "十年前", min(3, len(source_spans) - 1))
    watch_span = galgame_source_span_id(source_spans, "怀表", 0)
    final_span = galgame_source_span_id(source_spans, "十八分钟", len(source_spans) - 1)
    branch_lines = []
    for choice in previous["branch_design"]["choices"]:
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
    draft = task_input["previous_step"]["script_adaptation"]
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
    task_input = json_task_payload(combined)["input"]
    previous = task_input["previous_steps"]
    scene = previous["shooting_script_projection"]["scenes"][0]
    staged_scene = previous["character_staging"]["scene_lines"][0]
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
    return {
        "items": [
            {
                "name": "旧怀表",
                "description": "林岚随身携带、秒针曾逆行一圈的旧怀表",
                "description_en": "worn brass pocket watch with a counterclockwise second hand",
                "importance": "prop",
                "first_appearance_beat_ref": manga_beat_ref(combined),
                "visual_features": "黄铜外壳和磨损刻度必须连续一致；被林岚握紧时成为读者视线焦点并强化她的警觉",
                "source_span_ids": [source_span_id(combined, "旧怀表")],
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


def response_for(body: dict[str, Any]) -> str:
    messages = body.get("messages") or []
    combined = "\n".join(str(message.get("content", "")) for message in messages)
    if combined.strip() == "ping" or (len(messages) == 1 and "ping" in combined.lower()):
        return "pong"

    prompt_name = current_task_prompt_name(combined)
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
        return json.dumps(video_segment_plan_response(), ensure_ascii=False)

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
        return json.dumps(shooting_layout_response(), ensure_ascii=False)

    if "chapter_screenplay_skeleton" in combined:
        return json.dumps(shooting_skeleton_response(combined), ensure_ascii=False)

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

    if '"name": "implicit_attribute_classification"' in combined:
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

    return json.dumps(
        {
            "mock_unhandled": True,
            "message": "No deterministic fixture matched this production prompt.",
        },
        ensure_ascii=False,
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
        if normalized_path in {"/models", "/v1/models"}:
            self._json(200, {"object": "list", "data": [{"id": "a4n-e2e-model", "object": "model"}]})
            return
        if normalized_path == "/health":
            self._json(200, {"status": "ok"})
            return
        if normalized_path == "/fixtures/video.mp4":
            if self.server.video_base64 is None:
                self._json(503, {"error": {"message": "video fixture is not configured"}})
                return
            self._bytes(200, "video/mp4", base64.b64decode(self.server.video_base64))
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
        normalized_path = self.path.rstrip("/")

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
        if normalized_path in {"/images/generations", "/v1/images/generations"}:
            self._json(200, {"created": 0, "data": [{"b64_json": self.server.image_base64}]})
            return
        if normalized_path in {"/videos/generations", "/v1/videos/generations"}:
            if self.server.video_base64 is None:
                self._json(503, {"error": {"message": "video fixture is not configured"}})
                return
            self._json(
                200,
                {
                    "id": "a4n-e2e-video-job",
                    "data": [{"b64_json": self.server.video_base64, "format": "mp4"}],
                },
            )
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

        content = response_for(body)
        if body.get("stream") is True:
            self.send_response(200)
            self.send_header("Content-Type", "text/event-stream; charset=utf-8")
            self.send_header("Cache-Control", "no-cache")
            self.end_headers()
            for index in range(0, len(content), 1024):
                event = {
                    "choices": [
                        {
                            "index": 0,
                            "delta": {"content": content[index : index + 1024]},
                            "finish_reason": None,
                        }
                    ]
                }
                self.wfile.write(("data: " + json.dumps(event, ensure_ascii=False) + "\n\n").encode("utf-8"))
            final = {
                "choices": [{"index": 0, "delta": {}, "finish_reason": "stop"}],
                "usage": {"prompt_tokens": 10, "completion_tokens": 20, "total_tokens": 30},
            }
            self.wfile.write(("data: " + json.dumps(final) + "\n\n").encode("utf-8"))
            self.wfile.write(b"data: [DONE]\n\n")
            self.wfile.flush()
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


class ProviderServer(ThreadingHTTPServer):
    def __init__(
        self,
        address: tuple[str, int],
        log_path: Path,
        image_base64: str,
        video_base64: str | None,
    ):
        super().__init__(address, ProviderHandler)
        self.log_path = log_path
        self.image_base64 = image_base64
        self.video_base64 = video_base64
        self.log_lock = threading.Lock()
        self.request_index = 0

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
    args = parser.parse_args()
    args.log.parent.mkdir(parents=True, exist_ok=True)
    image_base64 = (
        base64.b64encode(args.image_file.read_bytes()).decode("ascii")
        if args.image_file
        else PNG_1X1_BASE64
    )
    video_base64 = (
        base64.b64encode(args.video_file.read_bytes()).decode("ascii")
        if args.video_file
        else None
    )
    ProviderServer(
        (args.host, args.port),
        args.log,
        image_base64,
        video_base64,
    ).serve_forever()


if __name__ == "__main__":
    main()
