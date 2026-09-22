"""Shooting-script and video-planning fixtures."""

from __future__ import annotations

import json
import re
from typing import Any

from mock_openai.fixtures.entities import source_span_id, source_span_pairs
from mock_openai.fixtures.reverse import embedded_json_objects
from mock_openai.prompt_parsing import json_task_payload
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
    source_text = dict(source_span_pairs(combined))
    return {
        "schema_version": "screenplay_unit_adaptation_v1",
        "narrative_unit_index": unit["narrative_unit_index"],
        "performance_facts": [
            {
                "fact_index": index,
                "shot_id": shot["segment_id"],
                "source_span_ids": [
                    span_id for span_id in unit["source_span_ids"]
                    if span_id in source_text and source_text[span_id].strip()
                    and source_text[span_id] in shot["prompt"]
                ],
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


def shooting_design_input(combined: str) -> tuple[dict[str, Any], dict[str, Any], str]:
    for value in embedded_json_objects(combined):
        locked = value.get("locked_visual_unit")
        screenplay = value.get("screenplay_plan_unit")
        if isinstance(locked, dict) and isinstance(screenplay, dict):
            target = value.get("target_shot")
            if not isinstance(target, dict) or not isinstance(target.get("segment_id"), str) or not target["segment_id"]:
                raise UnhandledPromptError("shooting design input has no target_shot.segment_id")
            return locked, screenplay, target["segment_id"]
    raise UnhandledPromptError("shooting design prompt contains no locked visual unit")


def shooting_design_response(combined: str) -> dict[str, Any]:
    locked, screenplay, target_id = shooting_design_input(combined)
    planned_shots = locked.get("shots")
    facts = screenplay.get("performance_facts")
    if not isinstance(planned_shots, list) or not planned_shots:
        raise UnhandledPromptError("locked visual unit contains no shots")
    if not isinstance(facts, list):
        raise UnhandledPromptError("screenplay plan unit contains no performance facts")

    facts_by_shot: list[list[tuple[dict[str, Any], list[str]]]] = [[] for _ in planned_shots]
    for fact in facts:
        if not isinstance(fact, dict) or not isinstance(fact.get("source_span_ids"), list):
            raise UnhandledPromptError("shooting performance fact is invalid")
        fact_shot_id = fact.get("shot_id")
        if not isinstance(fact_shot_id, str):
            raise UnhandledPromptError("shooting performance fact has no shot_id")
        shot_index = next(
            (
                index
                for index, planned in enumerate(planned_shots)
                if isinstance(planned, dict) and planned.get("segment_id") == fact_shot_id
            ),
            None,
        )
        if shot_index is None:
            raise UnhandledPromptError(
                f"shooting performance fact {fact.get('fact_index')} has no locked shot"
            )
        facts_by_shot[shot_index].append((fact, fact["source_span_ids"]))

    response = shooting_design_scene_template(combined)
    template_shot = response["shots"][0]
    template_beat = template_shot["performance_beats"][0]
    participants = screenplay.get("participants")
    if not isinstance(participants, list):
        raise UnhandledPromptError("screenplay plan unit participants are invalid")

    selected = [
        (planned, assigned_facts)
        for planned, assigned_facts in zip(planned_shots, facts_by_shot)
        if planned.get("segment_id") == target_id
    ]
    if len(selected) != 1:
        raise UnhandledPromptError("shooting target must identify exactly one locked shot")
    shots = []
    for shot_index, (planned, assigned_facts) in enumerate(selected, 1):
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
