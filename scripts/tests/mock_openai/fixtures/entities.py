"""Character, scene, object, and chapter-analysis fixtures."""

from __future__ import annotations

import re
from typing import Any

from mock_openai.fixtures.reverse import embedded_json_objects
from mock_openai.prompt_parsing import json_task_payload

def annotated_source_pairs(combined: str) -> list[tuple[str, str]]:
    pairs = []
    for value in embedded_json_objects(combined):
        content = value.get("content")
        if not isinstance(content, str):
            continue
        markers = list(re.finditer(r"⟦(ch\d+:s\d+:f\d+|\d+)⟧", content))
        for index, marker in enumerate(markers):
            end = markers[index + 1].start() if index + 1 < len(markers) else len(content)
            pairs.append((marker.group(1), content[marker.end():end]))
    return list(dict.fromkeys(pairs))

def source_span_pairs(combined: str) -> list[tuple[str, str]]:
    annotated = [(identifier, quote) for identifier, quote in annotated_source_pairs(combined) if identifier.startswith("ch")]
    structured = [(value["id"], value["text"]) for value in embedded_json_objects(combined)
                  if isinstance(value.get("id"), str) and re.fullmatch(r"ch\d+:s\d+:f\d+", value["id"])
                  and isinstance(value.get("text"), str)]
    return list(dict.fromkeys([*annotated, *structured]))


def source_span_id(combined: str, phrase: str, fallback_index: int = 0) -> str:
    pairs = source_span_pairs(combined)
    for span_id, text in pairs:
        if phrase in text:
            return span_id
    if not pairs:
        raise ValueError("chapter analysis prompt contains no source spans")
    return pairs[min(fallback_index, len(pairs) - 1)][0]


def text_anchor_id(combined: str, phrase: str) -> int:
    for identifier, quote in annotated_source_pairs(combined):
        if identifier.isdecimal() and phrase in quote:
            return int(identifier)
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
                    "assertion": "observed",
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
                    "assertion": "observed",
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
                    "assertion": "observed",
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
                    "assertion": "observed",
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
        "world_facts": [],
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
            "progressed": [],
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
    if not states or "observations_ref" not in states[-1]:
        raise ValueError("optimization request requires explicit observation references")
    records = [value["loop_observation"] for value in objects if isinstance(value.get("loop_observation"), dict)]
    if [record["sequence"] for record in records] != list(range(1, len(records) + 1)):
        raise ValueError("optimization observations are not an intact ordered history")
    observations = [record["value"] for record in records]
    if states[-1]["progress"]["observation_count"] != len(observations):
        raise ValueError("optimization request lost full observations")
    tasks = [json_task_payload(value["base_task"]) for value in objects
             if value.get("kind") == "base_task" and isinstance(value.get("base_task"), str)]
    if len(tasks) != 1:
        raise ValueError("optimization request requires one base task")
    current = tasks[0]["user_message"]
    if "A4N_OPTIMIZATION_PLAIN" in current:
        reply = "局部补丁提案需要用户确认后才会写回正文。"
        if "A4N_OPTIMIZATION_LONG_REPLY" in current:
            reply += "乙" * 6500 + "完整答复末尾"
        return {"thought": "该问题可直接回答。", "action": "final", "arguments": {"reply": reply}}
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
