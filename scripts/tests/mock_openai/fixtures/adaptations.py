"""Galgame and manga adaptation fixtures."""

from __future__ import annotations

import json
import re
from typing import Any

from mock_openai.fixtures.entities import source_span_id
from mock_openai.fixtures.reverse import embedded_json_objects
from mock_openai.prompt_parsing import json_task_payload
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
    source_span_ids = staged_scene["lines"][0]["source_span_ids"]
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
