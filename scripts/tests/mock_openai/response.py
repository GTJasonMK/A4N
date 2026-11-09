"""Prompt router for the deterministic provider."""

from __future__ import annotations

import hashlib
import base64
import json
import re
import time
from typing import Any

from mock_openai.fixtures.adaptations import (
    media_adaptation_response,
    galgame_branch_design_response,
    galgame_character_staging_response,
    galgame_script_adaptation_response,
    galgame_visual_planning_response,
    manga_drawing_prompt_response,
    manga_page_reference_discovery_response,
    manga_page_adaptation_response,
    manga_page_outline_response,
    manga_page_content_response,
    manga_page_storyboard_response,
    manga_preprocessing_response,
    manga_prompt_optimization_response,
)
from mock_openai.fixtures.entities import (
    character_discovery_response,
    character_sync_attribute_changes_response,
    character_sync_behaviors_response,
    character_sync_chapter_states_response,
    character_sync_evidence_events_response,
    chapter_analysis_character_state_response,
    chapter_analysis_facts_response,
    chapter_analysis_foreshadowing_response,
    chapter_analysis_summary_response,
    implicit_attribute_classification_response,
    object_discovery_response,
    optimization_analysis_response,
    optimization_chat_response,
    optimization_patch_response,
    scene_entity_state_response,
    scene_occurrence_response,
    scene_profile_candidates_response,
    scene_transition_response,
)
from mock_openai.fixtures.fact_feedback import fact_feedback_response, identity_feedback_response
from mock_openai.fixtures.reverse import (
    message_text,
    reverse_blueprint_response,
    reverse_outline_response,
    reverse_part_outline_response,
)
from mock_openai.fixtures.shooting import (
    chat_video_prompt_response,
    shooting_adaptation_response,
    shooting_design_response,
    shooting_layout_response,
    video_prompt_response,
    video_segment_plan_response,
    video_static_frame_response,
    video_storyboard_frame_response,
)
from mock_openai.fixtures.story import (
    blueprint_response,
    chapter_body_response,
    chapter_outline_response,
    inspiration_response,
    guided_inspiration_response,
    part_outline_response,
)
from mock_openai.prompt_parsing import current_task_prompt_name, json_task_payload
from mock_openai.protocol import UnhandledPromptError
from mock_openai.contract_guard import fixture_prompt_parts, validate_business_request

ANCHOR_OUTPUT_FIELDS = {
    "object_discovery": ("objects", "evidence_anchor_ids"),
    "character_discovery": ("characters", "evidence_anchor_id"),
    "scene_profile_candidates": ("scenes", "evidence_anchor_ids"),
    "scene_occurrence_extraction": ("occurrences", "source_anchor_ids"),
    "scene_transition_extraction": ("transitions", "source_anchor_ids"),
    "scene_entity_state_extraction": ("entity_states", "source_anchor_ids"),
    "character_sync_evidence_events": ("events", "evidence_anchor_id"),
    "character_sync_attribute_changes": ("attribute_changes", "evidence_anchor_id"),
    "character_sync_behavior_records": ("behaviors", "original_text_anchor_id"),
    "character_sync_deletion_candidates": ("deletion_candidates", "evidence_anchor_id"),
}


def normal_fixture_model(model: str) -> str:
    if model=="a4n-fact-feedback-invalid":return "a4n-fact-feedback"
    if model.startswith(("a4n-output-","a4n-anchor-","a4n-checkpoint-","a4n-scene-sync-","a4n-preprocessing-","a4n-page-planning-")):return "a4n-virtual-text"
    return model


def validated_fixture_response(body:dict,messages:list[dict]|None=None)->str:
    fixture=body if messages is None else {"messages":messages,"model":body["model"]}
    content=response_for(fixture)
    normal_model=normal_fixture_model(str(fixture.get("model","")))
    normal=content if normal_model==fixture.get("model") else response_for({**fixture,"model":normal_model})
    validate_business_request(body,fixture.get("messages",[]),normal)
    return content


def anchor_fault_response(body, fault, expected_root=None):
    combined = "\n".join(message_text(message.get("content", "")) for message in body.get("messages", []))
    selected = ANCHOR_OUTPUT_FIELDS.get(current_task_prompt_name(combined))
    raw = response_for({**body, "model": normal_fixture_model(str(body.get("model","")))})
    # Root names can overlap across unrelated contracts (facts also contain
    # characters). Inject only into the intended task, never into its upstream.
    if selected is None or (expected_root is not None and selected[0] != expected_root):
        return raw
    root, field = selected
    value = json.loads(raw)
    if value[root]:
        item = value[root][0]
        if fault == "missing_anchor":
            del item[field]
        elif fault == "string_ids":
            ids = item[field]
            item[field] = [str(anchor) for anchor in ids] if isinstance(ids, list) else str(ids)
        else:
            item[field] = [9223372036854775807] if isinstance(item[field], list) else 9223372036854775807
    return json.dumps(value, ensure_ascii=False)


def response_for(body: dict[str, Any]) -> str:
    messages = body.get("messages", [])
    original_messages = []
    for message in messages:
        content = message.get("content")
        try:
            value = json.loads(content) if isinstance(content, str) else None
        except ValueError:
            value = None
        if not isinstance(value, dict) or value.get("kind") != "model_output_correction":
            original_messages.append(message)
    if len(original_messages) != len(messages):
        return response_for({**body, "messages": original_messages, "_a4n_correction": True})
    model = str(body.get("model", ""))
    if model.startswith("a4n-page-planning-"):
        raw = response_for({**body, "model": "a4n-virtual-text"})
        value = json.loads(raw)
        if "action" in value and value.get("page_number") == 2:
            if model == "a4n-page-planning-fail-content" or (model == "a4n-page-planning-recover-content" and not body.get("_a4n_correction")):
                del value["reaction"]
        return json.dumps(value, ensure_ascii=False)
    if model.startswith("a4n-preprocessing-"):
        raw = response_for({**body, "model": "a4n-virtual-text"})
        combined = "\n".join(message_text(message.get("content", "")) for message in body.get("messages", []))
        step = current_task_prompt_name(combined)
        if step == "manga_preprocessing_speech" and (
            model == "a4n-preprocessing-fail-speech"
            or (model == "a4n-preprocessing-recover-speech" and not body.get("_a4n_correction"))
        ):
            return json.dumps({"speech": {"dialogues": [], "narrations": [{"source_event_id": "missing-event"}]}}, ensure_ascii=False)
        return raw
    if model.startswith("a4n-output-recover-"):
        fault = model.removeprefix("a4n-output-recover-")
        if fault not in {"string_ids", "missing_anchor", "unknown_anchor", "trailing-json"}:
            raise ValueError(f"unknown recovery fixture: {fault}")
        if body.get("_a4n_correction"):
            return response_for({**body, "model": "a4n-virtual-text"})
        if fault == "trailing-json":
            value = json.loads(response_for({**body, "model": "a4n-virtual-text"}))
            if value.get("schema_version") != "adaptation_plan_v1":
                raise ValueError("trailing JSON fixture requires an adaptation plan")
            value["intention"] = "a" * 26_000
            return json.dumps(value, ensure_ascii=False) + '{"unexpected":"TRAILING_ADAPTATION_MARKER"}'
        return anchor_fault_response(body, fault)
    if model.startswith("a4n-anchor-fail-"):
        root = model.removeprefix("a4n-anchor-fail-")
        if root not in {pair[0] for pair in ANCHOR_OUTPUT_FIELDS.values()}:
            raise ValueError(f"unknown anchor failure fixture: {root}")
        return anchor_fault_response(body, "string_ids", expected_root=root)
    if model.startswith("a4n-checkpoint-"):
        raw = response_for({**body, "model": normal_fixture_model(model)})
        value = json.loads(raw)
        fault = model.removeprefix("a4n-checkpoint-fail-")
        combined = "\n".join(message_text(message.get("content", "")) for message in body["messages"])
        if "patches" in value:
            from mock_openai.fixtures.reverse import embedded_json_objects
            content = next((item.get("content") for item in embedded_json_objects(combined)
                            if item.get("kind") == "chapter_content"), None)
            if not isinstance(content, str) or not content.strip():
                raise ValueError("checkpoint patch fixture requires the actual chapter content")
            old_text = next(line for line in content.splitlines() if line.strip())
            if content.count(old_text) != 1:
                raise ValueError("checkpoint patch fixture requires unique original text")
            value["patches"][0]["old_text"] = old_text
            value["patches"][0]["new_text"] = old_text + "林岚把信封攥得更紧。"
        if fault == "shooting-layout" and value.get("schema_version") == "screenplay_layout_unit_detail_v2":
            value["slugline"] = " "
        elif fault == "shooting-adaptation" and value.get("schema_version") == "screenplay_unit_adaptation_v1":
            value["performance_facts"][0]["actor"] = "不在参与者中的角色"
        elif "shots" in value and (
            fault == "shooting-design"
            or (fault == "shooting-design-second-shot" and value["shots"][0]["shot_id"] == "shot-2")
        ):
            value["shots"][0]["lens_intent"] = " "
        elif fault == "video-json" and "segments" in value and value["segments"][0]["source_shot_id"] == "shot-2":
            return json.dumps(value, ensure_ascii=False) + "]}"
        elif fault == "blueprint-characters" and isinstance(value.get("characters"), list):
            value["characters"].append(value["characters"][0].copy())
        elif fault == "inspiration" and "conversation_state" in value:
            del value["conversation_state"]["genre"]
        elif fault == "galgame-branch" and "choices" in value:
            value["choices"][0]["after_beat_ref"]["beat_id"] = "missing-beat"
        elif fault == "galgame-script" and "scene_lines" in value and "presentation" not in value["scene_lines"][0]["lines"][0]:
            value["scene_lines"][0]["lines"][0]["beat_ref"]["beat_id"] = "missing-beat"
        elif fault == "galgame-staging" and "scene_lines" in value and "presentation" in value["scene_lines"][0]["lines"][0]:
            value["scene_lines"][0]["lines"][0]["text"] = "未经允许改写的剧情"
        elif fault == "galgame-visual" and "backgrounds" in value:
            value["backgrounds"][0]["scene_index"] = 999
        elif fault == "manga-speech" and "visual_intent" in value and "第 2 页" in value["visual_intent"]:
            value["beat_refs"][0]["beat_id"] = "missing-beat"
        elif fault == "manga-visual" and "dominant_panel_number" in value:
            value["beat_refs"][0]["beat_id"] = "missing-beat"
        elif fault == "manga-items" and value.get("schema_version") == "manga_page_reference_discovery_v1":
            value["references"] = [{
                "target_kind": "object",
                "target_name": "旧怀表",
                "asset_kind": "generated_image",
                "asset_id": 999999,
                "usage": "沿用当前页面的怀表外观。",
            }]
        elif fault == "manga-outline" and value.get("schema_version") == "manga_page_outline_v1":
            value["pages"][0]["beat_refs"][0]["beat_id"] = "missing-beat"
        elif fault == "manga-page-content" and "action" in value and value.get("page_number") == 2:
            del value["reaction"]
        elif fault == "manga-page-adaptation" and "visual_intent" in value:
            value["beat_refs"][0]["beat_id"] = "missing-beat"
        elif fault == "manga-storyboard" and "dominant_panel_number" in value:
            value["page_number"] += 100
        elif fault == "manga-drawing" and value.get("schema_version") == "manga_page_drawing_prompt_v2" and value["page_number"] == 2:
            value["image_prompt"] = " "
        elif fault == "manga-optimization" and "image_prompt" in value:
            value["image_prompt"] = " "
        elif fault == "optimization-final" and value.get("action") == "final" and "issues" in value.get("arguments", {}):
            value["arguments"]["issues"][0]["arguments"]["evidence_span_id"] = "missing-span"
        elif fault == "optimization-patch" and "patches" in value:
            value["patches"][0]["old_text"] = "该文本并不存在于输入章节"
        return json.dumps(value, ensure_ascii=False)
    messages = body.get("messages") or []
    # System contracts may follow the dynamic task on Chat Completions. Keep
    # instructions before task data when handing text to input-payload parsers.
    system,combined,contracts=fixture_prompt_parts(messages)
    contract_name=contracts[0].get("name") if contracts else None
    if "你是 A4N 的媒体文本创作助手" in system:
        if not any(message.get("role")=="user" and message_text(message.get("content","")).strip() for message in messages):
            raise ValueError("text asset draft requires the current instruction")
        return "雨夜里，镜头从钟楼窗外缓缓推向门内。"
    if "你是 A4N 的视觉质量检查器" in system:
        images=[part.get("image_url",{}).get("url","") for message in messages if isinstance(message.get("content"),list) for part in message["content"] if part.get("type")=="image_url"]
        if not images:
            raise ValueError("visual inspection requires actual image bytes in the provider request")
        for uri in images:
            metadata,separator,data=uri.partition(",")
            if not separator or not metadata.startswith("data:image/") or not base64.b64decode(data,validate=True):
                raise ValueError("visual inspection image data URI is invalid")
        return "画面主体清楚，构图可读，未发现必须重新生成的缺陷。"
    if "A4N_AGENT_CANVAS_INPUT" in combined:
        return "镜头：雨水沿钟楼的窗棂滑落。"
    if combined.strip() == "ping" or (len(messages) == 1 and "ping" in combined.lower()):
        return "pong"

    prompt_name = current_task_prompt_name(combined)
    if "你负责把原作章节改编为指定媒介的叙事计划" in system:
        return json.dumps(media_adaptation_response(combined), ensure_ascii=False)
    if body.get("model") == "a4n-fact-feedback-identity" and "chapter_foreshadowing_events" in combined:
        return json.dumps(identity_feedback_response(combined), ensure_ascii=False)
    if body.get("model") in {"a4n-fact-feedback", "a4n-fact-feedback-invalid"}:
        return json.dumps(fact_feedback_response(combined, body["model"].endswith("invalid")), ensure_ascii=False)
    if "project_fact_consolidation" in combined:
        return json.dumps({"changes": []})
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
    if manga_prompt_name in {
        "manga_preprocessing_facts", "manga_preprocessing_speech",
        "manga_preprocessing_visuals", "manga_preprocessing_synthesis",
    }:
        fields = {
            "manga_preprocessing_facts": ("source_events", "characters"),
            "manga_preprocessing_speech": ("speech",),
            "manga_preprocessing_visuals": ("visual_anchors", "items"),
            "manga_preprocessing_synthesis": ("comic_synthesis",),
        }[manga_prompt_name]
        result = manga_preprocessing_response(combined)
        return json.dumps({field: result[field] for field in fields}, ensure_ascii=False)
    if manga_prompt_name == "manga_page_outline":
        return json.dumps(manga_page_outline_response(combined), ensure_ascii=False)
    if manga_prompt_name == "manga_page_content":
        return json.dumps(manga_page_content_response(combined), ensure_ascii=False)
    if manga_prompt_name == "manga_page_adaptation_plan":
        return json.dumps(manga_page_adaptation_response(combined), ensure_ascii=False)
    if manga_prompt_name == "manga_page_storyboard":
        return json.dumps(manga_page_storyboard_response(combined), ensure_ascii=False)
    if manga_prompt_name == "manga_page_reference_discovery":
        return json.dumps(manga_page_reference_discovery_response(combined), ensure_ascii=False)
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

    if "chapter_shooting_design_unit" in combined or "ShootingSceneV2" in combined:
        if model == "a4n-shooting-background-slow":
            time.sleep(6)
        elif model == "a4n-shooting-background-browser":
            time.sleep(20)
        return json.dumps(shooting_design_response(combined), ensure_ascii=False)

    if (
        "chapter_screenplay_unit_adaptation" in combined
        or "screenplay_unit_adaptation_v1" in combined
    ):
        return json.dumps(shooting_adaptation_response(combined), ensure_ascii=False)

    if (
        "chapter_screenplay_layout_unit" in combined
        or "screenplay_layout_unit_detail_v2" in combined
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

    if prompt_name == "object_discovery":
        return json.dumps(object_discovery_response(combined), ensure_ascii=False)

    if prompt_name == "character_discovery":
        return json.dumps(character_discovery_response(combined), ensure_ascii=False)

    if prompt_name == "scene_profile_candidates":
        return json.dumps(scene_profile_candidates_response(combined), ensure_ascii=False)

    if prompt_name == "scene_occurrence_extraction":
        result = scene_occurrence_response(combined)
        missing_state = {
            "a4n-scene-sync-missing-entry": "entry_state",
            "a4n-scene-sync-missing-exit": "exit_state",
            "a4n-scene-sync-recover-exit": "exit_state",
        }.get(model)
        if missing_state and not (model == "a4n-scene-sync-recover-exit" and body.get("_a4n_correction")):
            del result["occurrences"][0][missing_state]
        timeline_fixtures = {
            "a4n-scene-sync-number": 1,
            "a4n-scene-sync-object": {"order": 1},
            "a4n-scene-sync-null": None,
        }
        if body.get("model") in timeline_fixtures:
            result["occurrences"][0]["timeline_position"] = timeline_fixtures[body["model"]]
        if body.get("model") == "a4n-scene-sync-reference":
            result["occurrences"][0]["scene_ref"] = "missing_scene"
        if body.get("model") == "a4n-scene-sync-coverage":
            result["occurrences"] = result["occurrences"][:1]
        return json.dumps(result, ensure_ascii=False)

    if prompt_name == "scene_transition_extraction":
        return json.dumps(scene_transition_response(combined), ensure_ascii=False)

    if prompt_name == "scene_entity_state_extraction":
        return json.dumps(scene_entity_state_response(combined), ensure_ascii=False)

    if prompt_name == "character_sync_evidence_events":
        return json.dumps(character_sync_evidence_events_response(combined), ensure_ascii=False)

    if prompt_name == "character_sync_attribute_changes":
        return json.dumps(character_sync_attribute_changes_response(combined), ensure_ascii=False)

    if prompt_name == "character_sync_chapter_states":
        if body.get("model") == "a4n-character-sync-cancel":
            # Leave a valid call in flight while the CLI cancels its owner.
            time.sleep(10)
        return json.dumps(character_sync_chapter_states_response(combined), ensure_ascii=False)

    if prompt_name == "character_sync_behavior_records":
        return json.dumps(character_sync_behaviors_response(combined), ensure_ascii=False)

    if prompt_name == "character_sync_deletion_candidates":
        return json.dumps({"deletion_candidates": []}, ensure_ascii=False)

    if prompt_name == "implicit_classification":
        return json.dumps(implicit_attribute_classification_response(combined), ensure_ascii=False)

    stage_match = re.search(r'"(?:stage_key|key)"\s*:\s*"(story_core|world|characters|relationships)"', combined)
    if stage_match:
        return json.dumps(blueprint_response(stage_match.group(1)), ensure_ascii=False)

    part_match = re.search(
        r'"generation_request"\s*:\s*\{[^{}]*"part_number"\s*:\s*(\d+)',
        combined,
        re.DOTALL,
    )
    if part_match and (contract_name=="part_outline_single" or ("character_focus" in combined and "key_events" in combined)):
        return json.dumps(part_outline_response(int(part_match.group(1))), ensure_ascii=False)

    chapter_matches = re.findall(
        r'"wait_to_generate"\s*:\s*\{[^{}]*"start_chapter"\s*:\s*(\d+)',
        combined,
        re.DOTALL,
    )
    if chapter_matches and (contract_name=="outline" or ("word_count_target" in combined and "pacing_score" in combined)):
        return json.dumps(
            {"chapters": [chapter_outline_response(int(chapter_matches[-1]))]},
            ensure_ascii=False,
        )

    if "## 正文字数要求" in combined and "不要输出 word_count" in combined:
        return chapter_body_response()

    if prompt_name == "chapter_scene_generation":
        return chapter_body_response()

    if "content_optimization_chat_agent_loop" in combined:
        return json.dumps(optimization_chat_response(combined), ensure_ascii=False)

    if "content_optimization_agent_loop" in combined:
        return json.dumps(optimization_analysis_response(combined), ensure_ascii=False)

    if "content_optimization_patch" in combined and "old_text_must_match_once" in combined:
        return json.dumps(optimization_patch_response(), ensure_ascii=False)

    if "小说概念大师·文思" in system:
        if model == "a4n-creation-journey":
            round_number = 1 + sum(message.get("role") == "assistant" for message in messages)
            return json.dumps(guided_inspiration_response(round_number), ensure_ascii=False)
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
