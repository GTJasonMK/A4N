"""Prompt router for the deterministic provider."""

from __future__ import annotations

import hashlib
import json
import re
import time
from typing import Any

from mock_openai_fixture_adaptations import (
    galgame_branch_design_response,
    galgame_character_staging_response,
    galgame_script_adaptation_response,
    galgame_visual_planning_response,
    manga_character_binding_response,
    manga_drawing_prompt_response,
    manga_item_response,
    manga_page_plan_response,
    manga_prompt_optimization_response,
    manga_speech_selection_response,
    manga_synthesis_response,
    manga_visual_anchor_response,
)
from mock_openai_fixture_entities import (
    source_span_id,
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
from mock_openai_fixture_reverse import (
    cache_fixture_usage,
    message_text,
    reverse_blueprint_response,
    reverse_outline_response,
    reverse_part_outline_response,
)
from mock_openai_fixture_shooting import (
    chat_video_prompt_response,
    shooting_adaptation_response,
    shooting_design_response,
    shooting_layout_response,
    video_prompt_response,
    video_segment_plan_response,
    video_static_frame_response,
    video_storyboard_frame_response,
)
from mock_openai_fixture_story import (
    blueprint_response,
    chapter_body_response,
    chapter_outline_response,
    inspiration_response,
    part_outline_response,
)
from mock_openai_fixture_reverse import agent_chat_fixture
from mock_openai_prompt_parsing import current_task_prompt_name
from mock_openai_provider_protocol import UnhandledPromptError
def response_for(body: dict[str, Any]) -> str:
    model = str(body.get("model", ""))
    if model.startswith("a4n-checkpoint-"):
        raw = response_for({**body, "model": "a4n-virtual-text"})
        value = json.loads(raw)
        fault = model.removeprefix("a4n-checkpoint-fail-")
        combined = "\n".join(message_text(message.get("content", "")) for message in body["messages"])
        if value.get("schema_version") == "screenplay_unit_adaptation_v1":
            for fact in value["performance_facts"]:
                fact["source_span_ids"] = [source_span_id(combined, "进入地下机房", 0)]
        if "patches" in value:
            from mock_openai_fixture_reverse import embedded_json_objects
            content = next((item.get("content") for item in embedded_json_objects(combined)
                            if item.get("kind") == "chapter_content"), None)
            if not isinstance(content, str) or not content.strip():
                raise ValueError("checkpoint patch fixture requires the actual chapter content")
            old_text = next(line for line in content.splitlines() if line.strip())
            if content.count(old_text) != 1:
                raise ValueError("checkpoint patch fixture requires unique original text")
            value["patches"][0]["old_text"] = old_text
            value["patches"][0]["new_text"] = old_text + "林岚把信封攥得更紧。"
        if fault == "shooting-layout" and value.get("schema_version") == "screenplay_layout_unit_detail_v1":
            value["slugline"] = " "
        elif fault == "shooting-adaptation" and value.get("schema_version") == "screenplay_unit_adaptation_v1":
            value["performance_facts"][0]["actor"] = "不在参与者中的角色"
        elif fault == "shooting-design" and "shots" in value:
            value["shots"][0]["lens_intent"] = " "
        elif fault == "blueprint-characters" and isinstance(value.get("characters"), list):
            value["characters"].append(value["characters"][0].copy())
        elif fault == "galgame-branch" and "choices" in value:
            value["choices"][0]["after_beat_ref"]["beat_index"] = 999
        elif fault == "galgame-script" and "scene_lines" in value and "presentation" not in value["scene_lines"][0]["lines"][0]:
            value["scene_lines"][0]["lines"][0]["beat_ref"]["beat_index"] = 999
        elif fault == "galgame-staging" and "scene_lines" in value and "presentation" in value["scene_lines"][0]["lines"][0]:
            value["scene_lines"][0]["lines"][0]["text"] = "未经允许改写的剧情"
        elif fault == "galgame-visual" and "backgrounds" in value:
            value["backgrounds"][0]["scene_index"] = 999
        elif fault == "manga-speech" and "dialogues" in value:
            (value["dialogues"] or value["narrations"])[0]["beat_ref"]["beat_index"] = 999
        elif fault == "manga-visual" and "scenes" in value:
            value["scenes"][0]["beat_refs"][0]["beat_index"] = 999
        elif fault == "manga-items" and "items" in value:
            value["items"][0]["first_appearance_beat_ref"]["beat_index"] = 999
        elif fault == "manga-optimization" and "image_prompt" in value:
            value["image_prompt"] = " "
        elif fault == "optimization-final" and value.get("action") == "final" and "issues" in value.get("arguments", {}):
            value["arguments"]["issues"][0]["arguments"]["evidence_span_id"] = "missing-span"
        elif fault == "optimization-patch" and "patches" in value:
            value["patches"][0]["old_text"] = "该文本并不存在于输入章节"
        return json.dumps(value, ensure_ascii=False)
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
        result = scene_occurrence_response(combined)
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

    if '"name": "scene_transition_extraction"' in combined:
        return json.dumps(scene_transition_response(combined), ensure_ascii=False)

    if '"name": "scene_entity_state_extraction"' in combined:
        return json.dumps(scene_entity_state_response(combined), ensure_ascii=False)

    if '"name": "character_sync_evidence_events"' in combined:
        return json.dumps(character_sync_evidence_events_response(combined), ensure_ascii=False)

    if '"name": "character_sync_attribute_changes"' in combined:
        return json.dumps(character_sync_attribute_changes_response(combined), ensure_ascii=False)

    if '"name": "character_sync_chapter_states"' in combined:
        if body.get("model") == "a4n-character-sync-cancel":
            # Leave a valid call in flight while the CLI cancels its owner.
            time.sleep(10)
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
