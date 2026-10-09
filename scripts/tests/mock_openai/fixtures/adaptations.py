"""Galgame and manga adaptation fixtures."""

from __future__ import annotations

import json
import re
from typing import Any

from mock_openai.fixtures.entities import source_marker_pairs, source_span_id
from mock_openai.fixtures.reverse import embedded_json_objects
from mock_openai.prompt_parsing import json_task_payload
from mock_openai.protocol import UnhandledPromptError


def media_adaptation_response(combined: str) -> dict[str, Any]:
    payloads = [
        value
        for value in embedded_json_objects(combined)
        if value.get("target") in {"manga", "galgame"}
        and isinstance(value.get("chapter"), dict)
        and isinstance(value["chapter"].get("content"), str)
    ]
    if not payloads:
        raise UnhandledPromptError("adaptation prompt contains no source chapter")
    payload = payloads[-1]
    spans = [marker for marker, _ in source_marker_pairs(payload["chapter"]["content"])]
    if not spans:
        raise UnhandledPromptError("adaptation prompt contains no source evidence")
    result = {
        "schema_version": "adaptation_plan_v1", "target": payload["target"],
        "intention": "用可验证的异常推动林岚与周衡的关系变化", "decisions": [],
        "units": [
            {
                "id": "station", "origin": "adapted", "title": "旧车站的七分钟", "location": "旧车站通道",
                "participants": ["林岚", "周衡"], "dramatic_question": "眼前的异常是否真实", "objective": "确认怀表与来信的关联",
                "obstacle": "周衡隐瞒过去", "entry_state": "林岚留在通道入口", "exit_state": "林岚决定进入机房", "source_span_ids": spans,
                "beats": [{"id": "evidence", "origin": "adapted", "kind": "dialogue", "importance": "required", "is_climax": True,
                    "action": "林岚核对怀表并追问周衡", "reaction": "周衡收紧工具箱提手", "dialogue_intent": "围绕齿轮暗记追问十年前的秘密",
                    "entry_state": "双方彼此试探", "exit_state": "林岚决定独自查证", "source_span_ids": spans}],
            },
            {
                "id": "machine-room", "origin": "adapted", "title": "地下机房的回声", "location": "地下机房入口",
                "participants": ["林岚", "周衡"], "dramatic_question": "被隐藏的事故能否被还原", "objective": "进入机房核对第二份证据",
                "obstacle": "机房门禁和周衡的沉默", "entry_state": "林岚走到机房门口", "exit_state": "林岚取得可验证的线索", "source_span_ids": spans,
                "beats": [{"id": "machine-room-reveal", "origin": "adapted", "kind": "action", "importance": "required", "is_climax": False,
                    "action": "林岚跨过门槛检查机房里的齿轮记录", "reaction": "周衡停在通道后方没有阻拦", "dialogue_intent": "让沉默的证据先于解释出现",
                    "entry_state": "机房门半掩，灯光不稳", "exit_state": "林岚找到事故记录", "source_span_ids": spans}],
            },
        ],
    }
    if payload["target"] == "manga":
        result["page_count"] = 2
        result["units"] = result["units"][:1]
    return result


def galgame_beat_ref(scene: dict[str, Any]) -> dict[str, Any]:
    beat = scene["beats"][0]
    return {
        "source_chapter_id": int(scene["source_chapter_id"]),
        "unit_id": scene["unit_id"],
        "beat_id": beat["id"],
    }


def galgame_shared_step(combined: str, name: str) -> dict[str, Any]:
    for value in embedded_json_objects(combined):
        step = value.get(name)
        if isinstance(step, dict) and "$ref" not in step:
            return step
    raise UnhandledPromptError(f"galgame prompt contains no shared {name}")


def galgame_branch_design_response(combined: str) -> dict[str, Any]:
    scenes = galgame_shared_step(combined, "adaptation_projection")["scenes"]
    if len(scenes) < 2:
        return {"choices": []}
    scene = scenes[0]
    beat_ref = galgame_beat_ref(scene)
    returns_to_scene = scenes[1]
    returns_to = {"source_chapter_id": returns_to_scene["source_chapter_id"], "unit_id": returns_to_scene["unit_id"]}
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
                        "returns_to": returns_to,
                    },
                    {
                        "branch_id": "question_zhou",
                        "option_text": "追问齿轮暗记",
                        "branch_title": "周衡的停顿",
                        "branch_objective": "确认周衡与十年前事故的关系",
                        "dramatic_action": "林岚把信封暗记转向周衡",
                        "reaction": "周衡避开视线，回答前明显停顿",
                        "player_knowledge": ["周衡认识齿轮暗记并隐瞒了信息"],
                        "returns_to": returns_to,
                    },
                ],
            }
        ]
    }


def galgame_source_span_id(chapter_content: str, phrase: str, fallback_index: int) -> str:
    source_spans = source_marker_pairs(chapter_content)
    for marker, text in source_spans:
        if phrase in text:
            return marker
    return source_spans[fallback_index][0]


def galgame_script_adaptation_response(combined: str) -> dict[str, Any]:
    task_input = json_task_payload(combined)["input"]
    scenes = galgame_shared_step(combined, "adaptation_projection")["scenes"]
    scene = scenes[0]
    branch_design = galgame_shared_step(combined, "branch_design")
    beat_ref = galgame_beat_ref(scene)
    chapter_content = task_input["chapters"][0]["content"]
    source_spans = source_marker_pairs(chapter_content)
    first_span = galgame_source_span_id(chapter_content, "雨", 0)
    zhou_span = galgame_source_span_id(chapter_content, "十年前", min(3, len(source_spans) - 1))
    watch_span = galgame_source_span_id(chapter_content, "怀表", 0)
    final_span = galgame_source_span_id(chapter_content, "十八分钟", len(source_spans) - 1)
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
    result = {
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
            },
            *[{
                "scene_index": int(follow_up_scene["index"]),
                "lines": [
                    {
                        "index": 1,
                        "beat_ref": galgame_beat_ref(follow_up_scene),
                        "line_type": "stage_direction",
                        "speaker_name": "",
                        "text": "林岚跨过机房门槛，冷绿指示灯照出墙后的齿轮记录。",
                        "source_span_ids": [final_span],
                    },
                    {
                        "index": 2,
                        "beat_ref": galgame_beat_ref(follow_up_scene),
                        "line_type": "dialogue",
                        "speaker_name": "林岚",
                        "text": "记录还在，十年前的事故可以被重新核对。",
                        "source_span_ids": [watch_span],
                    },
                ],
            } for follow_up_scene in scenes[1:]],
        ],
        "branch_scene_lines": branch_lines,
    }
    for scene_lines in [*result["scene_lines"], *result["branch_scene_lines"]]:
        for line in scene_lines["lines"]:
            line["origin"] = "adapted"
    return result


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
        "origin": line["origin"],
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
    scenes = galgame_shared_step(combined, "adaptation_projection")["scenes"]
    staged_scenes = galgame_shared_step(combined, "character_staging")["scene_lines"]
    scene = scenes[0]
    staged_scene = staged_scenes[0]
    source_span_ids = staged_scene["lines"][0]["source_span_ids"]
    cg_line = staged_scene["lines"][-1]
    return {
        "backgrounds": [
            {
                "scene_index": scene["index"],
                "background_note": "旧车站封闭通道，夜间停电，冷灰广告屏反光与地下机房冷绿指示灯形成纵深，墙面保留齿轮暗记。",
                "scene_profile_id": None,
                "source_span_ids": [source_span_ids[0]],
            },
            *[{
                "scene_index": follow_up_scene["index"],
                "background_note": "地下机房入口，冷绿指示灯照出齿轮记录，林岚在前景核对证据，周衡停在通道后方。",
                "scene_profile_id": None,
                "source_span_ids": [follow_up_staged_scene["lines"][0]["source_span_ids"][0]],
            } for follow_up_scene, follow_up_staged_scene in zip(scenes[1:], staged_scenes[1:], strict=True)],
        ],
        "event_cgs": [
            {
                "scene_index": scene["index"],
                "line_index": cg_line["index"],
                "cg_note": "林岚合上逆行怀表，在墙面留下齿轮暗记后跨入地下机房；周衡提着工具箱停在后方。",
                "importance": "high",
                "source_span_ids": cg_line["source_span_ids"],
            },
            *[{
                "scene_index": follow_up_scene["index"],
                "line_index": follow_up_staged_scene["lines"][-1]["index"],
                "cg_note": "林岚在地下机房找到齿轮记录，冷绿灯光把证据与她的怀表并置在同一画面中。",
                "importance": "high",
                "source_span_ids": follow_up_staged_scene["lines"][-1]["source_span_ids"],
            } for follow_up_scene, follow_up_staged_scene in zip(scenes[1:], staged_scenes[1:], strict=True)],
        ],
    }


def manga_beat_ref(combined: str) -> dict[str, str]:
    for value in reversed(embedded_json_objects(combined)):
        if all(isinstance(value.get(field), str) and value[field] for field in ("unit_id", "beat_id")):
            return {"unit_id": value["unit_id"], "beat_id": value["beat_id"]}
    raise UnhandledPromptError("manga prompt contains no local adaptation beat")


def manga_preprocessing_response(combined: str) -> dict[str, Any]:
    source_id = source_span_id(combined, "林岚")
    return {
        "schema_version": "manga_preprocessing_v3",
        "source_events": {
            "events": [{
                "event_id": "event-1",
                "type": "action",
                "description": "林岚在旧车站通道检查逆行怀表。",
                "participants": ["林岚", "周衡"],
                "importance": "critical",
                "source_span_ids": [source_id],
            }],
        },
        "characters": {
            "林岚": {
                "name": "林岚",
                "appearance": "黑色短发，深色风衣，右手握旧怀表",
                "personality": "克制警觉",
                "role": "protagonist",
                "gender": "female",
                "age_description": "青年",
                "source_span_ids": [source_id],
            },
            "周衡": {
                "name": "周衡",
                "appearance": "深色工作夹克，提旧工具箱",
                "personality": "表面从容，隐瞒旧事",
                "role": "supporting",
                "gender": "male",
                "age_description": "青年",
                "source_span_ids": [source_id],
            },
        },
        "speech": {"dialogues": [], "narrations": []},
        "visual_anchors": {
            "scenes": [{
                "index": 0,
                "location": "旧车站通道",
                "location_en": "station passage",
                "time_of_day": "night",
                "atmosphere": "两人保持距离，机房入口形成纵深",
                "weather": None,
                "lighting": "冷灰顶灯",
                "indoor_outdoor": "indoor",
                "description": "林岚在前景检查怀表，周衡停在通道后方。",
                "source_event_ids": ["event-1"],
                "source_span_ids": [source_id],
            }],
        },
        "items": {"items": []},
        "comic_synthesis": {
            "chapter_summary": "林岚用怀表确认车站的时间异常，并察觉周衡隐瞒过去。",
            "mood_progression": ["警觉观察", "异常坐实", "主动查证"],
        },
    }


def manga_page_reference_discovery_response(combined: str) -> dict[str, Any]:
    task = json_task_payload(combined)
    payload = task.get("input")
    if not isinstance(payload, dict):
        raise UnhandledPromptError("manga reference discovery prompt contains no task input")
    page_number = payload.get("page_number")
    if (
        not isinstance(page_number, int)
        or isinstance(page_number, bool)
        or page_number <= 0
    ):
        raise UnhandledPromptError("manga reference discovery prompt contains no valid page number")
    assets = payload.get("available_assets")
    if not isinstance(assets, list):
        raise UnhandledPromptError("manga reference discovery prompt contains no available assets")
    references = []
    for asset in assets:
        if not isinstance(asset, dict):
            raise UnhandledPromptError("manga reference discovery prompt contains an invalid asset")
        if "怀表" not in str(asset.get("description", "")):
            continue
        references.append({
            "target_kind": "object",
            "target_name": "旧怀表",
            "asset_kind": asset.get("asset_kind"),
            "asset_id": asset.get("asset_id"),
            "usage": "沿用怀表的黄铜外壳与磨损刻度。",
        })
        break
    return {
        "schema_version": "manga_page_reference_discovery_v1",
        "page_number": page_number,
        "references": references,
    }


def manga_page_slots_response(combined: str) -> dict[str, Any]:
    adaptation = next(
        (
            value.get("adaptation_plan")
            for value in reversed(embedded_json_objects(combined))
            if isinstance(value.get("adaptation_plan"), dict)
        ),
        None,
    )
    expected_page_count = adaptation.get("page_count") if adaptation else None
    if expected_page_count != 2:
        raise UnhandledPromptError("manga page slots fixture expects a two-page adaptation plan")
    beat_ref = manga_beat_ref(combined)
    source_id = source_span_id(combined, "林岚")
    content = [
        {
            "summary": "林岚在旧车站通道核对逆行怀表。",
            "source_span_ids": [source_id],
            "beat_refs": [beat_ref],
            "action": "林岚对照怀表和未来来信确认时间异常。",
            "reaction": "周衡避开她的视线。",
            "dialogue_intent": "确认齿轮暗记与十年前事故的关系。",
            "entry_state": "林岚停在通道入口。",
            "exit_state": "林岚确认怀表发生逆行。",
        },
        {
            "summary": "林岚拒绝交出信件，决定进入地下机房。",
            "source_span_ids": [source_id],
            "beat_refs": [beat_ref],
            "action": "林岚收起信件并跨过机房门槛。",
            "reaction": "周衡停在通道后方，没有跟上。",
            "dialogue_intent": None,
            "entry_state": "林岚确认怀表发生逆行。",
            "exit_state": "林岚进入地下机房查证。",
        },
    ]
    return {
        "schema_version": "manga_page_slots_v1",
        "pages": [
            {"page_number": index + 1, "content": page}
            for index, page in enumerate(content)
        ],
    }


def manga_page_content(combined: str) -> dict[str, Any]:
    for value in reversed(embedded_json_objects(combined)):
        content = value.get("page_content")
        if isinstance(content, dict) and isinstance(content.get("summary"), str):
            return content
    raise UnhandledPromptError("manga page prompt contains no page_content")


def manga_page_adaptation_response(combined: str) -> dict[str, Any]:
    page_number = _manga_page_number(combined)
    content = manga_page_content(combined)
    return {
        "visual_intent": f"第 {page_number} 页以怀表和人物距离强调本页剧情转折。",
        "continuity_requirements": ["林岚的怀表和深色风衣保持一致。"],
        "retained_details": [content["action"]],
        "omitted_details": [],
        "beat_refs": content["beat_refs"],
    }


def manga_page_storyboard_response(combined: str) -> dict[str, Any]:
    page_number = _manga_page_number(combined)
    content = manga_page_content(combined)
    adaptation = next(
        (
            value["page_adaptation"]
            for value in reversed(embedded_json_objects(combined))
            if isinstance(value.get("page_adaptation"), dict)
        ),
        None,
    )
    if not isinstance(adaptation, dict):
        raise UnhandledPromptError("manga storyboard prompt contains no page_adaptation")
    beat_refs = adaptation["beat_refs"]
    return {
        "page_number": page_number,
        "beat_refs": beat_refs,
        "role": "setup" if page_number == 1 else "rising",
        "pacing": "medium",
        "importance": "high",
        "reading_flow": "top_to_bottom_left_to_right",
        "page_goal": content["summary"],
        "visual_thesis": adaptation["visual_intent"],
        "entry_state": content["entry_state"],
        "exit_state": content["exit_state"],
        "dominant_panel_number": 1,
        "panels": [
            {
                "panel_number": 1,
                "beat_refs": beat_refs,
                "function": "action",
                "size": "dominant",
                "shot_scale": "medium",
                "camera_angle": "eye_level",
                "participants": ["林岚", "周衡"],
                "composition": "林岚位于前景，周衡落后半步，地下机房入口位于右侧纵深。",
                "observable_action": content["action"],
                "reaction": content["reaction"],
                "spatial_relationship": "林岚靠近墙面，周衡停在通道后方。",
                "continuity_from_previous": None,
                "text_layers": [],
            }
        ],
    }


def _manga_page_number(combined: str) -> int:
    page_numbers = re.findall(r'"page_number"\s*:\s*(\d+)', combined)
    return int(page_numbers[-1]) if page_numbers else 1


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
