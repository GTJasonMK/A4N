use std::collections::HashSet;

use a4n_core::error::AppError;
use serde_json::Value;

use crate::prompt_language::PromptOutputLanguage;

use super::{
    VIDEO_PROMPT_DIALOGUE_BEAT_FIELDS, VideoFramePrompt, VideoPromptShot, VideoSegmentPlan,
};

#[derive(Debug, Clone, Copy)]
enum PromptTextKind {
    VideoVisual,
    VideoMotion,
    FramePrompt,
    FrameNegative,
}

#[derive(Debug, Clone, Copy)]
enum DialogueTracePolicy {
    ModelOutput,
    InternalNormalized,
}

const API_PROMPT_UPLOAD_STATUS_TERMS: &[&str] = &[
    "无输入首帧",
    "没有输入首帧",
    "无输入图片",
    "没有输入图片",
    "无输入尾帧",
    "没有输入尾帧",
    "无首帧",
    "没有首帧",
    "无首帧图",
    "没有首帧图",
    "无尾帧",
    "没有尾帧",
    "无尾帧图",
    "没有尾帧图",
    "无上传图片",
    "本段无上传图片",
    "没有上传图片",
    "无参考图",
    "没有参考图",
    "仅依赖文字描述",
];
pub(super) fn validate_shot(
    shot: VideoPromptShot,
    allowed_source_span_ids: &[String],
    prompt_output_language: PromptOutputLanguage,
) -> Result<VideoPromptShot, AppError> {
    validate_shot_with_dialogue_policy(
        shot,
        allowed_source_span_ids,
        prompt_output_language,
        DialogueTracePolicy::InternalNormalized,
    )
}

pub(super) fn validate_model_shot(
    shot: VideoPromptShot,
    allowed_source_span_ids: &[String],
    prompt_output_language: PromptOutputLanguage,
) -> Result<VideoPromptShot, AppError> {
    validate_shot_with_dialogue_policy(
        shot,
        allowed_source_span_ids,
        prompt_output_language,
        DialogueTracePolicy::ModelOutput,
    )
}

fn validate_shot_with_dialogue_policy(
    shot: VideoPromptShot,
    allowed_source_span_ids: &[String],
    prompt_output_language: PromptOutputLanguage,
    dialogue_trace_policy: DialogueTracePolicy,
) -> Result<VideoPromptShot, AppError> {
    if shot.shot_number < 1 {
        return Err(AppError::InvalidArgument(
            "视频提示词 shot_number 必须大于 0".to_string(),
        ));
    }
    if shot.title.trim().is_empty() {
        return Err(AppError::InvalidArgument(
            "视频提示词 title 不能为空".to_string(),
        ));
    }
    if shot.scene_summary.trim().is_empty() {
        return Err(AppError::InvalidArgument(
            "视频提示词 scene_summary 不能为空".to_string(),
        ));
    }
    validate_self_contained_context(&shot.self_contained_context, shot.shot_number)?;
    validate_structured_value(&shot.start_state, "视频提示词 start_state")?;
    validate_structured_value(&shot.end_state, "视频提示词 end_state")?;
    validate_transition_in(&shot.transition_in)?;
    let visual_prompt = required_min_prompt_text(
        shot.visual_prompt,
        "视频提示词 visual_prompt",
        prompt_output_language,
        PromptTextKind::VideoVisual,
    )?;
    let motion_prompt = required_min_prompt_text(
        shot.motion_prompt,
        "视频提示词 motion_prompt",
        prompt_output_language,
        PromptTextKind::VideoMotion,
    )?;
    validate_prompt_has_no_upload_status(&visual_prompt, "视频提示词 visual_prompt")?;
    validate_prompt_has_required_sections(
        &visual_prompt,
        "视频提示词 visual_prompt",
        "visual_prompt",
        prompt_output_language,
    )?;
    validate_prompt_has_no_upload_status(&motion_prompt, "视频提示词 motion_prompt")?;
    validate_prompt_has_required_sections(
        &motion_prompt,
        "视频提示词 motion_prompt",
        "motion_prompt",
        prompt_output_language,
    )?;
    validate_motion_prompt_has_no_temporal_storyboard(&motion_prompt)?;
    validate_optional_prompt_text(&shot.negative_prompt, "视频提示词 negative_prompt")?;
    validate_structured_value(&shot.blocking_plan, "视频提示词 blocking_plan")?;
    validate_structured_value(&shot.lighting_plan, "视频提示词 lighting_plan")?;
    validate_structured_value(
        &shot.emotion_action_beats,
        "视频提示词 emotion_action_beats",
    )?;
    let dialogue_beats = validate_dialogue_beats(
        &shot.dialogue_beats,
        allowed_source_span_ids,
        dialogue_trace_policy,
    )?;
    validate_structured_value(&shot.cinematography_plan, "视频提示词 cinematography_plan")?;
    validate_temporal_beats(&shot.temporal_beats, shot.duration_secs)?;
    validate_reference_binding(&shot.reference_binding, &shot.reference_image_ids)?;
    validate_structured_value(&shot.continuity_check, "视频提示词 continuity_check")?;
    if !(1..=60).contains(&shot.duration_secs) {
        return Err(AppError::InvalidArgument(
            "视频提示词 duration_secs 必须在 1 到 60 秒之间".to_string(),
        ));
    }
    if shot.aspect_ratio.trim().is_empty() {
        return Err(AppError::InvalidArgument(
            "视频提示词 aspect_ratio 不能为空".to_string(),
        ));
    }
    validate_reference_ids(&shot.reference_image_ids, "视频提示词 reference_image_ids")?;
    Ok(VideoPromptShot {
        title: shot.title.trim().to_string(),
        scene_summary: shot.scene_summary.trim().to_string(),
        visual_prompt,
        motion_prompt,
        negative_prompt: shot.negative_prompt.trim().to_string(),
        aspect_ratio: shot.aspect_ratio.trim().to_string(),
        dialogue_beats,
        ..shot
    })
}

pub(super) fn validate_prompt_has_no_upload_status(
    value: &str,
    field: &str,
) -> Result<(), AppError> {
    if let Some(term) = video_prompt_upload_status_term(value) {
        return Err(AppError::InvalidArgument(format!(
            "{field} 不能写上传状态说明“{term}”；有输入帧时描述输入帧画面，没有输入帧时直接描述文字起点或上一段尾态"
        )));
    }
    Ok(())
}

fn validate_prompt_has_required_sections(
    value: &str,
    field: &str,
    field_key: &str,
    prompt_output_language: PromptOutputLanguage,
) -> Result<(), AppError> {
    if let Some(section) = video_prompt_stale_section_for_language(field_key, value) {
        return Err(AppError::InvalidArgument(format!(
            "{field} 使用旧版视频提示词章节 {section}；请重新生成或优化为当前格式"
        )));
    }
    if let Some(section) =
        video_prompt_missing_required_section_for_language(field_key, value, prompt_output_language)
    {
        return Err(AppError::InvalidArgument(format!(
            "{field} 缺少必需章节 {section}"
        )));
    }
    if field_key == "visual_prompt" {
        validate_prompt_has_shot_storyboard_section(value, field, prompt_output_language)?;
    }
    Ok(())
}

pub(super) fn validate_prompt_has_shot_storyboard_section(
    value: &str,
    field: &str,
    prompt_output_language: PromptOutputLanguage,
) -> Result<(), AppError> {
    let sections = prompt_output_language.visual_sections();
    let Some(section_title) = sections.get(2).copied() else {
        return Ok(());
    };
    let section = prompt_section_content(value, section_title, sections).ok_or_else(|| {
        AppError::InvalidArgument(format!("{field} 缺少必需章节 {section_title}"))
    })?;
    if !contains_shot_number_marker(section, prompt_output_language) {
        let example = match prompt_output_language {
            PromptOutputLanguage::ZhCn => "镜头 1：",
            PromptOutputLanguage::En => "Shot 1:",
        };
        return Err(AppError::InvalidArgument(format!(
            "{field} 的 {section_title} 必须使用镜头编号组织，例如“{example}”，不能只写动作时间段"
        )));
    }
    if contains_time_table_marker(section) {
        return Err(AppError::InvalidArgument(format!(
            "{field} 的 {section_title} 必须按镜头编号描述运镜、动作和空间变化，不能写成“总时长/0-3秒/2-4 seconds”这类精确时间表"
        )));
    }
    Ok(())
}

fn prompt_section_content<'a>(
    text: &'a str,
    section_title: &str,
    sections: &[&str],
) -> Option<&'a str> {
    let start = text.find(section_title)?;
    let content_start = start + section_title.len();
    let rest = &text[content_start..];
    let end = sections
        .iter()
        .filter(|section| **section != section_title)
        .filter_map(|section| rest.find(section))
        .min()
        .unwrap_or(rest.len());
    Some(rest[..end].trim())
}

fn contains_shot_number_marker(text: &str, prompt_output_language: PromptOutputLanguage) -> bool {
    match prompt_output_language {
        PromptOutputLanguage::ZhCn => ["镜头 1", "镜头1", "镜头 一", "镜头一", "镜头 １", "镜头１"]
            .iter()
            .any(|marker| text.contains(marker)),
        PromptOutputLanguage::En => {
            let lower = text.to_ascii_lowercase();
            ["shot 1", "shot1", "shot one"]
                .iter()
                .any(|marker| lower.contains(marker))
        }
    }
}

fn contains_time_table_marker(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    text.contains("总时长") || lower.contains("total duration") || contains_time_range_marker(text)
}

fn contains_time_range_marker(text: &str) -> bool {
    let chars = text.char_indices().collect::<Vec<_>>();
    let mut i = 0usize;
    while i < chars.len() {
        let (_, ch) = chars[i];
        if !ch.is_ascii_digit() {
            i += 1;
            continue;
        }
        i += 1;
        while i < chars.len() && (chars[i].1.is_ascii_digit() || chars[i].1 == '.') {
            i += 1;
        }
        while i < chars.len() && chars[i].1.is_whitespace() {
            i += 1;
        }
        if i >= chars.len() || !matches!(chars[i].1, '-' | '–' | '—') {
            continue;
        }
        i += 1;
        while i < chars.len() && chars[i].1.is_whitespace() {
            i += 1;
        }
        if i >= chars.len() || !chars[i].1.is_ascii_digit() {
            continue;
        }
        i += 1;
        while i < chars.len() && (chars[i].1.is_ascii_digit() || chars[i].1 == '.') {
            i += 1;
        }
        let lookahead_start = chars.get(i).map(|(index, _)| *index).unwrap_or(text.len());
        let lookahead_end = chars
            .get((i + 12).min(chars.len()))
            .map(|(index, _)| *index)
            .unwrap_or(text.len());
        let lookahead = text[lookahead_start..lookahead_end].to_ascii_lowercase();
        if lookahead.contains('秒') || lookahead.contains("sec") || lookahead.contains("second") {
            return true;
        }
    }
    false
}

fn validate_motion_prompt_has_no_temporal_storyboard(value: &str) -> Result<(), AppError> {
    if video_motion_prompt_has_temporal_storyboard(value) {
        return Err(AppError::InvalidArgument(
            "视频提示词 motion_prompt 不应重复镜头分镜；镜头编号分镜只写在 visual_prompt 的 [镜头分镜] 中"
                .to_string(),
        ));
    }
    Ok(())
}

fn validate_self_contained_context(value: &Value, shot_number: i32) -> Result<(), AppError> {
    let object = value.as_object().ok_or_else(|| {
        AppError::InvalidArgument("视频提示词 self_contained_context 必须是对象".to_string())
    })?;
    if object.is_empty() {
        return Err(AppError::InvalidArgument(
            "视频提示词 self_contained_context 不能为空".to_string(),
        ));
    }
    for key in [
        "must_repeat_from_previous",
        "must_establish_in_segment",
        "must_preserve_throughout",
        "handoff_to_next",
        "omission_risks",
    ] {
        if shot_number == 1 && key == "must_repeat_from_previous" && object.contains_key(key) {
            continue;
        }
        validate_structured_value(
            object.get(key).unwrap_or(&Value::Null),
            &format!("视频提示词 self_contained_context.{key}"),
        )?;
    }
    Ok(())
}

fn validate_transition_in(value: &Value) -> Result<(), AppError> {
    let object = value.as_object().ok_or_else(|| {
        AppError::InvalidArgument("视频提示词 transition_in 必须是对象".to_string())
    })?;
    if object.is_empty() {
        return Err(AppError::InvalidArgument(
            "视频提示词 transition_in 不能为空".to_string(),
        ));
    }
    for key in [
        "from_previous_end",
        "to_current_start",
        "cut_type",
        "camera_transition",
        "first_beat_action",
    ] {
        validate_structured_value(
            object.get(key).unwrap_or(&Value::Null),
            &format!("视频提示词 transition_in.{key}"),
        )?;
    }
    Ok(())
}

pub(super) fn validate_segment_plan(
    segment: VideoSegmentPlan,
) -> Result<VideoSegmentPlan, AppError> {
    let VideoSegmentPlan {
        segment_number,
        analysis_shot_id,
        segment_index_in_shot,
        shot_segment_count,
        covered_beat_ids,
        title,
        scene_summary,
        blocking_summary,
        spatial_anchors,
        emotion_action_goals,
        video_intent,
        emotional_arc,
        performance_plan,
        material_coverage,
        source_span_ids,
        duration_secs,
        aspect_ratio,
        reference_image_ids,
    } = segment;
    if segment_number < 1 {
        return Err(AppError::InvalidArgument(
            "视频分段规划 segment_number 必须大于 0".to_string(),
        ));
    }
    let title = required_text(title, "视频分段规划 title")?;
    let scene_summary = required_text(scene_summary, "视频分段规划 scene_summary")?;
    let blocking_summary = required_text(blocking_summary, "视频分段规划 blocking_summary")?;
    validate_segment_spatial_anchors(&spatial_anchors)?;
    validate_segment_emotion_action_goals(&emotion_action_goals)?;
    validate_segment_video_intent(&video_intent)?;
    validate_segment_emotional_arc(&emotional_arc)?;
    validate_duration(duration_secs, "视频分段规划 duration_secs")?;
    validate_segment_performance_plan(&performance_plan, duration_secs)?;
    validate_segment_material_coverage(&material_coverage)?;
    if source_span_ids.is_empty() {
        return Err(AppError::InvalidArgument(
            "视频分段规划 source_span_ids 不能为空".to_string(),
        ));
    }
    let aspect_ratio = required_text(aspect_ratio, "视频分段规划 aspect_ratio")?;
    validate_covered_beat_ids(&covered_beat_ids)?;
    Ok(VideoSegmentPlan {
        segment_number,
        analysis_shot_id,
        segment_index_in_shot,
        shot_segment_count,
        covered_beat_ids,
        title,
        scene_summary,
        blocking_summary,
        spatial_anchors,
        emotion_action_goals,
        video_intent,
        emotional_arc,
        performance_plan,
        material_coverage,
        source_span_ids,
        duration_secs,
        aspect_ratio,
        reference_image_ids,
    })
}

fn validate_segment_material_coverage(value: &Value) -> Result<(), AppError> {
    validate_structured_value(value, "视频分段规划 material_coverage")?;
    let object = value.as_object().ok_or_else(|| {
        AppError::InvalidArgument("视频分段规划 material_coverage 必须是 JSON 对象".to_string())
    })?;
    required_string_array(
        object.get("required_entities"),
        "视频分段规划 material_coverage.required_entities",
        false,
    )?;
    required_string_array(
        object.get("missing_or_weak_references"),
        "视频分段规划 material_coverage.missing_or_weak_references",
        true,
    )?;
    required_object_text(
        object.get("risk_note"),
        "视频分段规划 material_coverage.risk_note",
    )?;
    if value
        .as_object()
        .is_some_and(|object| object.contains_key("covered_by_reference_image_ids"))
    {
        return Err(AppError::InvalidArgument(
            "视频分段规划 material_coverage 不能输出 covered_by_reference_image_ids；片段规划只做素材需求审计，不绑定具体图片 ID"
                .to_string(),
        ));
    }
    Ok(())
}

fn validate_segment_spatial_anchors(value: &Value) -> Result<(), AppError> {
    let items = required_value_array(value, "视频分段规划 spatial_anchors")?;
    for (index, item) in items.iter().enumerate() {
        let field = format!("视频分段规划 spatial_anchors[{}]", index + 1);
        let object = item
            .as_object()
            .ok_or_else(|| AppError::InvalidArgument(format!("{field} 必须是 JSON 对象")))?;
        required_object_text(object.get("anchor"), &format!("{field}.anchor"))?;
        required_object_text(object.get("usage"), &format!("{field}.usage"))?;
    }
    Ok(())
}

fn validate_segment_emotion_action_goals(value: &Value) -> Result<(), AppError> {
    let items = required_value_array(value, "视频分段规划 emotion_action_goals")?;
    for (index, item) in items.iter().enumerate() {
        let field = format!("视频分段规划 emotion_action_goals[{}]", index + 1);
        let object = item
            .as_object()
            .ok_or_else(|| AppError::InvalidArgument(format!("{field} 必须是 JSON 对象")))?;
        required_object_text(object.get("character"), &format!("{field}.character"))?;
        required_object_text(object.get("emotion"), &format!("{field}.emotion"))?;
        required_object_text(
            object.get("visible_action"),
            &format!("{field}.visible_action"),
        )?;
    }
    Ok(())
}

fn validate_segment_video_intent(value: &Value) -> Result<(), AppError> {
    validate_structured_value(value, "视频分段规划 video_intent")?;
    let object = value.as_object().ok_or_else(|| {
        AppError::InvalidArgument("视频分段规划 video_intent 必须是 JSON 对象".to_string())
    })?;
    for field in ["hook", "audience_effect", "rhythm", "visual_priority"] {
        required_object_text(
            object.get(field),
            &format!("视频分段规划 video_intent.{field}"),
        )?;
    }
    Ok(())
}

fn validate_segment_emotional_arc(value: &Value) -> Result<(), AppError> {
    validate_structured_value(value, "视频分段规划 emotional_arc")?;
    let object = value.as_object().ok_or_else(|| {
        AppError::InvalidArgument("视频分段规划 emotional_arc 必须是 JSON 对象".to_string())
    })?;
    for field in ["start", "turn", "end", "function"] {
        required_object_text(
            object.get(field),
            &format!("视频分段规划 emotional_arc.{field}"),
        )?;
    }
    Ok(())
}

fn validate_segment_performance_plan(value: &Value, duration_secs: i32) -> Result<(), AppError> {
    validate_structured_value(value, "视频分段规划 performance_plan")?;
    let object = value.as_object().ok_or_else(|| {
        AppError::InvalidArgument("视频分段规划 performance_plan 必须是 JSON 对象".to_string())
    })?;
    for field in [
        "movement_path",
        "body_performance",
        "gaze_and_hands",
        "pause_or_reaction",
        "camera_relationship",
    ] {
        required_object_text(
            object.get(field),
            &format!("视频分段规划 performance_plan.{field}"),
        )?;
    }
    let time_blocks = required_object_array(
        object.get("time_blocks"),
        "视频分段规划 performance_plan.time_blocks",
    )?;
    validate_segment_performance_time_blocks(time_blocks, duration_secs)?;
    let action_script = required_object_array(
        object.get("action_script"),
        "视频分段规划 performance_plan.action_script",
    )?;
    validate_segment_performance_action_script(action_script)?;
    Ok(())
}

fn validate_segment_performance_time_blocks(
    blocks: &[Value],
    duration_secs: i32,
) -> Result<(), AppError> {
    let mut expected_start = 0.0_f64;
    for (index, block) in blocks.iter().enumerate() {
        let field = format!("视频分段规划 performance_plan.time_blocks[{}]", index + 1);
        let object = block
            .as_object()
            .ok_or_else(|| AppError::InvalidArgument(format!("{field} 必须是 JSON 对象")))?;
        let start_sec =
            required_object_number(object.get("start_sec"), &format!("{field}.start_sec"))?;
        let end_sec = required_object_number(object.get("end_sec"), &format!("{field}.end_sec"))?;
        if (start_sec - expected_start).abs() > 0.05 {
            return Err(AppError::InvalidArgument(format!(
                "{field}.start_sec 必须从 {expected_start:.2} 秒连续开始，实际为 {start_sec:.2}"
            )));
        }
        if end_sec <= start_sec {
            return Err(AppError::InvalidArgument(format!(
                "{field}.end_sec 必须大于 start_sec"
            )));
        }
        if end_sec - f64::from(duration_secs) > 0.05 {
            return Err(AppError::InvalidArgument(format!(
                "{field}.end_sec 不能超过 duration_secs"
            )));
        }
        required_object_text(object.get("camera"), &format!("{field}.camera"))?;
        required_object_text(
            object.get("subject_action"),
            &format!("{field}.subject_action"),
        )?;
        required_object_text(
            object.get("continuity_target"),
            &format!("{field}.continuity_target"),
        )?;
        expected_start = end_sec;
    }
    if (expected_start - f64::from(duration_secs)).abs() > 0.05 {
        return Err(AppError::InvalidArgument(
            "视频分段规划 performance_plan.time_blocks 最后一段 end_sec 必须等于 duration_secs"
                .to_string(),
        ));
    }
    Ok(())
}

fn validate_segment_performance_action_script(items: &[Value]) -> Result<(), AppError> {
    for (index, item) in items.iter().enumerate() {
        let field = format!("视频分段规划 performance_plan.action_script[{}]", index + 1);
        let object = item
            .as_object()
            .ok_or_else(|| AppError::InvalidArgument(format!("{field} 必须是 JSON 对象")))?;
        for key in ["character", "body", "movement", "hands", "gaze", "timing"] {
            required_object_text(object.get(key), &format!("{field}.{key}"))?;
        }
    }
    Ok(())
}

fn validate_covered_beat_ids(ids: &[i32]) -> Result<(), AppError> {
    let mut seen = HashSet::new();
    for id in ids {
        if *id < 1 {
            return Err(AppError::InvalidArgument(
                "视频分段规划 covered_beat_ids 不能包含小于 1 的 beat_id".to_string(),
            ));
        }
        if !seen.insert(*id) {
            return Err(AppError::InvalidArgument(format!(
                "视频分段规划 covered_beat_ids 包含重复 beat_id：{id}"
            )));
        }
    }
    Ok(())
}

pub(super) fn validate_frame_prompt(
    segment: VideoFramePrompt,
    prompt_output_language: PromptOutputLanguage,
) -> Result<VideoFramePrompt, AppError> {
    let VideoFramePrompt {
        frame_number,
        title,
        scene_summary,
        storyboard_panels,
        frame_prompt,
        blocking_state,
        lighting_state,
        negative_prompt,
        duration_secs,
        aspect_ratio,
        reference_image_ids,
        segment_numbers,
    } = segment;
    if frame_number < 1 {
        return Err(AppError::InvalidArgument(
            "视频首尾帧提示词 frame_number 必须大于 0".to_string(),
        ));
    }
    let title = required_text(title, "视频首尾帧提示词 title")?;
    let scene_summary = required_text(scene_summary, "视频首尾帧提示词 scene_summary")?;
    let frame_prompt = required_min_prompt_text(
        frame_prompt,
        "视频首尾帧提示词 frame_prompt",
        prompt_output_language,
        PromptTextKind::FramePrompt,
    )?;
    validate_structured_value(&blocking_state, "视频首尾帧提示词 blocking_state")?;
    validate_structured_value(&lighting_state, "视频首尾帧提示词 lighting_state")?;
    let negative_prompt = required_min_prompt_text(
        negative_prompt,
        "视频首尾帧提示词 negative_prompt",
        prompt_output_language,
        PromptTextKind::FrameNegative,
    )?;
    validate_duration(duration_secs, "视频首尾帧提示词 duration_secs")?;
    let aspect_ratio = required_text(aspect_ratio, "视频首尾帧提示词 aspect_ratio")?;
    validate_reference_ids(&reference_image_ids, "视频首尾帧提示词 reference_image_ids")?;
    validate_reference_label_text(
        &frame_prompt,
        "视频首尾帧提示词 frame_prompt",
        reference_image_ids.len(),
    )?;
    validate_reference_label_text(
        &negative_prompt,
        "视频首尾帧提示词 negative_prompt",
        reference_image_ids.len(),
    )?;
    validate_segment_numbers(&segment_numbers, "视频首尾帧提示词 segment_numbers")?;
    Ok(VideoFramePrompt {
        frame_number,
        title,
        scene_summary,
        storyboard_panels,
        frame_prompt,
        blocking_state,
        lighting_state,
        negative_prompt,
        duration_secs,
        aspect_ratio,
        reference_image_ids,
        segment_numbers,
    })
}

fn required_text(value: String, field: &str) -> Result<String, AppError> {
    let text = value.trim();
    if text.is_empty() {
        return Err(AppError::InvalidArgument(format!("{field} 不能为空")));
    }
    Ok(text.to_string())
}

fn required_min_prompt_text(
    value: String,
    field: &str,
    prompt_output_language: PromptOutputLanguage,
    kind: PromptTextKind,
) -> Result<String, AppError> {
    let text = required_text(value, field)?;
    validate_prompt_text_density(&text, field, prompt_output_language, kind)?;
    validate_direct_prompt_text(&text, field)?;
    Ok(text)
}

fn validate_optional_prompt_text(value: &str, field: &str) -> Result<(), AppError> {
    let text = value.trim();
    if contains_structured_prompt_text(text) {
        return Err(AppError::InvalidArgument(format!(
            "{field} 不能包含 JSON、字典、数组或字段名片段"
        )));
    }
    validate_direct_prompt_text(text, field)?;
    Ok(())
}

fn validate_prompt_text_density(
    text: &str,
    field: &str,
    prompt_output_language: PromptOutputLanguage,
    kind: PromptTextKind,
) -> Result<(), AppError> {
    match prompt_output_language {
        PromptOutputLanguage::ZhCn => validate_min_chars(text, field, zh_min_chars(kind)),
        PromptOutputLanguage::En => validate_min_english_density(
            text,
            field,
            english_min_words(kind),
            english_min_chars(kind),
        ),
    }
}

fn validate_min_chars(text: &str, field: &str, min_chars: usize) -> Result<(), AppError> {
    let char_count = text.chars().count();
    if char_count >= min_chars {
        return Ok(());
    }
    Err(AppError::InvalidArgument(format!(
        "{field} 过短，至少需要 {min_chars} 个字符，当前 {char_count} 个字符"
    )))
}

fn validate_min_english_density(
    text: &str,
    field: &str,
    min_words: usize,
    min_chars: usize,
) -> Result<(), AppError> {
    let word_count = text
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .filter(|word| !word.is_empty())
        .count();
    let char_count = text.chars().count();
    if word_count >= min_words || char_count >= min_chars {
        return Ok(());
    }
    Err(AppError::InvalidArgument(format!(
        "{field} 过短，英文输出至少需要 {min_words} 个英文词或 {min_chars} 个字符，当前 {word_count} 个英文词、{char_count} 个字符"
    )))
}

fn zh_min_chars(kind: PromptTextKind) -> usize {
    match kind {
        PromptTextKind::VideoVisual | PromptTextKind::FramePrompt => 80,
        PromptTextKind::VideoMotion => 45,
        PromptTextKind::FrameNegative => 20,
    }
}

fn english_min_words(kind: PromptTextKind) -> usize {
    match kind {
        PromptTextKind::VideoVisual | PromptTextKind::FramePrompt => 60,
        PromptTextKind::VideoMotion => 30,
        PromptTextKind::FrameNegative => 8,
    }
}

fn english_min_chars(kind: PromptTextKind) -> usize {
    match kind {
        PromptTextKind::VideoVisual => 320,
        PromptTextKind::VideoMotion => 180,
        PromptTextKind::FramePrompt => 350,
        PromptTextKind::FrameNegative => 60,
    }
}

pub fn validate_direct_prompt_text(text: &str, field: &str) -> Result<(), AppError> {
    if text.trim().is_empty() {
        return Ok(());
    }
    if let Some(term) = direct_prompt_internal_reference_term(text) {
        return Err(AppError::InvalidArgument(format!(
            "{field} 不能在自然语言提示词中写内部图片 ID 或内部字段“{term}”；只能按后端参考槽位顺序写“参考图 1”“参考图 2”，不要写 image_id、图片 ID、available_assets、候选素材标签或原始图片数字 ID"
        )));
    }
    Ok(())
}

pub fn video_prompt_upload_status_term(text: &str) -> Option<&'static str> {
    API_PROMPT_UPLOAD_STATUS_TERMS
        .iter()
        .copied()
        .find(|term| text.contains(term))
}

pub fn video_prompt_missing_required_section_for_language(
    field: &str,
    text: &str,
    prompt_output_language: PromptOutputLanguage,
) -> Option<&'static str> {
    let required_sections = match field {
        "visual_prompt" => prompt_output_language.visual_sections(),
        "motion_prompt" => prompt_output_language.motion_sections(),
        "frame_prompt" => prompt_output_language.frame_sections(),
        _ => return None,
    };
    let mut cursor = 0;
    for section in required_sections {
        let Some(offset) = text[cursor..].find(section) else {
            return Some(section);
        };
        cursor += offset + section.len();
    }
    None
}

pub fn video_motion_prompt_has_temporal_storyboard(text: &str) -> bool {
    text.lines().any(|line| {
        let text = line.trim();
        matches!(
            text,
            "[片段内时间分布]"
                | "【片段内时间分布】"
                | "[时序分镜]"
                | "【时序分镜】"
                | "[镜头分镜]"
                | "【镜头分镜】"
                | "[Timing Breakdown]"
                | "【Timing Breakdown】"
                | "[Temporal Storyboard]"
                | "【Temporal Storyboard】"
                | "[Shot Storyboard]"
                | "【Shot Storyboard】"
        )
    })
}

pub fn video_prompt_stale_section_for_language(field: &str, text: &str) -> Option<&'static str> {
    let stale_sections: &[&str] = match field {
        "visual_prompt" => &[
            "[输入图片介绍]",
            "【输入图片介绍】",
            "[片段内时间分布]",
            "【片段内时间分布】",
            "[时序分镜]",
            "【时序分镜】",
            "[Input Image Notes]",
            "【Input Image Notes】",
            "[Timing Breakdown]",
            "【Timing Breakdown】",
            "[Temporal Storyboard]",
            "【Temporal Storyboard】",
        ],
        "motion_prompt" => &[
            "[片段内时间分布]",
            "【片段内时间分布】",
            "[时序分镜]",
            "【时序分镜】",
            "[连续性要求]",
            "【连续性要求】",
            "[Timing Breakdown]",
            "【Timing Breakdown】",
            "[Temporal Storyboard]",
            "【Temporal Storyboard】",
            "[Continuity Requirements]",
            "【Continuity Requirements】",
        ],
        _ => &[],
    };
    stale_sections
        .iter()
        .copied()
        .find(|section| text.contains(section))
}

pub(in crate::video) fn direct_prompt_internal_reference_term(text: &str) -> Option<&'static str> {
    let lower = text.to_ascii_lowercase();
    if lower.contains("image_id") {
        return Some("image_id");
    }
    if lower.contains("image id") {
        return Some("image id");
    }
    if lower.contains("available_assets") {
        return Some("available_assets");
    }
    if lower.contains("candidate_label") {
        return Some("candidate_label");
    }
    if lower.contains("source_display_label") {
        return Some("source_display_label");
    }
    if lower.contains("manual_library_asset") {
        return Some("manual_library_asset_*");
    }
    if lower.contains("asset_id") {
        return Some("asset_id");
    }
    if lower.contains("source_id") {
        return Some("source_id");
    }
    if text.contains("候选素材") {
        return Some("候选素材标签");
    }
    if contains_compact_reference_number(text, "参考图") {
        return Some("参考图后紧贴数字");
    }
    if contains_compact_reference_number(text, "图片") {
        return Some("图片后紧贴数字");
    }
    if contains_compact_reference_number(text, "素材图") {
        return Some("素材图后紧贴数字");
    }
    if text.contains("参考图 #") {
        return Some("参考图 #");
    }
    if text.contains("图片 #") {
        return Some("图片 #");
    }
    if text.contains("素材图 #") {
        return Some("素材图 #");
    }
    if lower.contains("图片id") {
        return Some("图片 ID");
    }
    if lower.contains("图片 id") {
        return Some("图片 ID");
    }
    if lower.contains("素材id") {
        return Some("素材 ID");
    }
    if lower.contains("素材 id") {
        return Some("素材 ID");
    }
    if lower.contains("参考图id") {
        return Some("参考图 ID");
    }
    if lower.contains("参考图 id") {
        return Some("参考图 ID");
    }
    None
}

fn contains_compact_reference_number(text: &str, marker: &str) -> bool {
    for (index, _) in text.match_indices(marker) {
        let after_marker = &text[index + marker.len()..];
        if after_marker
            .chars()
            .next()
            .is_some_and(|ch| ch.is_ascii_digit())
        {
            return true;
        }
    }
    false
}

pub fn validate_reference_label_text(
    text: &str,
    field: &str,
    reference_count: usize,
) -> Result<(), AppError> {
    for (index, _) in text.match_indices("参考图") {
        let after_marker = &text[index + "参考图".len()..];
        let mut chars = after_marker.char_indices();
        let mut saw_whitespace = false;
        let digit_start = loop {
            match chars.next() {
                Some((_, ch)) if ch.is_whitespace() => {
                    saw_whitespace = true;
                }
                Some((index, ch)) if ch.is_ascii_digit() => break Some(index),
                _ => break None,
            }
        };
        let Some(digit_start) = digit_start else {
            continue;
        };
        if !saw_whitespace {
            return Err(AppError::InvalidArgument(format!(
                "{field} 的参考图编号必须按实际输入顺序写成“参考图 1”“参考图 2”，不能写成“参考图12”或内部图片 ID"
            )));
        }
        let digits = after_marker[digit_start..]
            .chars()
            .take_while(|ch| ch.is_ascii_digit())
            .collect::<String>();
        if digits.len() > 1 && digits.starts_with('0') {
            return Err(AppError::InvalidArgument(format!(
                "{field} 的参考图编号不能使用前导 0：参考图 {digits}"
            )));
        }
        let number = digits.parse::<usize>().map_err(|err| {
            AppError::InvalidArgument(format!("{field} 的参考图编号解析失败：{err}"))
        })?;
        if number == 0 || number > reference_count {
            let allowed = if reference_count == 0 {
                "当前没有绑定参考图槽位，不能写任何“参考图 N”".to_string()
            } else {
                format!(
                    "当前后端参考槽位绑定了 {reference_count} 张，只能写参考图 1 到参考图 {reference_count}"
                )
            };
            return Err(AppError::InvalidArgument(format!(
                "{field} 引用了不存在的参考图 {number}；{allowed}"
            )));
        }
    }
    Ok(())
}

fn contains_structured_prompt_text(text: &str) -> bool {
    text.contains('{')
        || text.contains('}')
        || text.contains("[{")
        || text.contains("}]")
        || text.contains("\":")
        || text.contains("':")
}

fn validate_structured_value(value: &Value, field: &str) -> Result<(), AppError> {
    match value {
        Value::Null => Err(AppError::InvalidArgument(format!("{field} 不能为空"))),
        Value::String(text) if text.trim().is_empty() => {
            Err(AppError::InvalidArgument(format!("{field} 不能为空")))
        }
        Value::Array(items) if items.is_empty() => {
            Err(AppError::InvalidArgument(format!("{field} 不能为空")))
        }
        Value::Object(map) if map.is_empty() => {
            Err(AppError::InvalidArgument(format!("{field} 不能为空")))
        }
        _ => Ok(()),
    }
}

fn required_object_text(value: Option<&Value>, field: &str) -> Result<(), AppError> {
    let Some(Value::String(text)) = value else {
        return Err(AppError::InvalidArgument(format!(
            "{field} 必须是非空字符串"
        )));
    };
    if text.trim().is_empty() {
        return Err(AppError::InvalidArgument(format!("{field} 不能为空")));
    }
    Ok(())
}

fn required_object_array<'a>(
    value: Option<&'a Value>,
    field: &str,
) -> Result<&'a [Value], AppError> {
    let Some(Value::Array(items)) = value else {
        return Err(AppError::InvalidArgument(format!("{field} 必须是非空数组")));
    };
    if items.is_empty() {
        return Err(AppError::InvalidArgument(format!("{field} 不能为空")));
    }
    Ok(items)
}

fn required_value_array<'a>(value: &'a Value, field: &str) -> Result<&'a [Value], AppError> {
    let Value::Array(items) = value else {
        return Err(AppError::InvalidArgument(format!("{field} 必须是非空数组")));
    };
    if items.is_empty() {
        return Err(AppError::InvalidArgument(format!("{field} 不能为空")));
    }
    Ok(items)
}

fn required_string_array(
    value: Option<&Value>,
    field: &str,
    allow_empty: bool,
) -> Result<(), AppError> {
    let Some(Value::Array(items)) = value else {
        return Err(AppError::InvalidArgument(format!("{field} 必须是数组")));
    };
    if !allow_empty && items.is_empty() {
        return Err(AppError::InvalidArgument(format!("{field} 不能为空")));
    }
    for (index, item) in items.iter().enumerate() {
        let Some(_) = item.as_str().map(str::trim).filter(|text| !text.is_empty()) else {
            return Err(AppError::InvalidArgument(format!(
                "{field}[{}] 必须是非空字符串",
                index + 1
            )));
        };
    }
    Ok(())
}

fn required_object_number(value: Option<&Value>, field: &str) -> Result<f64, AppError> {
    let Some(number) = value.and_then(Value::as_f64) else {
        return Err(AppError::InvalidArgument(format!("{field} 必须是数字")));
    };
    if !number.is_finite() {
        return Err(AppError::InvalidArgument(format!("{field} 必须是有限数字")));
    }
    Ok(number)
}

fn validate_temporal_beats(value: &Value, duration_secs: i32) -> Result<(), AppError> {
    let beats = value.as_array().ok_or_else(|| {
        AppError::InvalidArgument("视频提示词 temporal_beats 必须是数组".to_string())
    })?;
    if beats.is_empty() {
        return Err(AppError::InvalidArgument(
            "视频提示词 temporal_beats 不能为空".to_string(),
        ));
    }

    let mut expected_start = 0.0_f64;
    for (index, beat) in beats.iter().enumerate() {
        let object = beat.as_object().ok_or_else(|| {
            AppError::InvalidArgument(format!(
                "视频提示词 temporal_beats[{}] 必须是对象",
                index + 1
            ))
        })?;
        let start_sec = object
            .get("start_sec")
            .and_then(Value::as_f64)
            .ok_or_else(|| {
                AppError::InvalidArgument(format!(
                    "视频提示词 temporal_beats[{}].start_sec 必须是数字",
                    index + 1
                ))
            })?;
        let end_sec = object
            .get("end_sec")
            .and_then(Value::as_f64)
            .ok_or_else(|| {
                AppError::InvalidArgument(format!(
                    "视频提示词 temporal_beats[{}].end_sec 必须是数字",
                    index + 1
                ))
            })?;
        if (start_sec - expected_start).abs() > 0.05 {
            return Err(AppError::InvalidArgument(format!(
                "视频提示词 temporal_beats[{}].start_sec 必须衔接上一段 end_sec",
                index + 1
            )));
        }
        if end_sec <= start_sec {
            return Err(AppError::InvalidArgument(format!(
                "视频提示词 temporal_beats[{}].end_sec 必须大于 start_sec",
                index + 1
            )));
        }
        if end_sec - f64::from(duration_secs) > 0.05 {
            return Err(AppError::InvalidArgument(format!(
                "视频提示词 temporal_beats[{}].end_sec 不能超过 duration_secs",
                index + 1
            )));
        }
        validate_structured_value(
            object.get("camera").unwrap_or(&Value::Null),
            &format!("视频提示词 temporal_beats[{}].camera", index + 1),
        )?;
        validate_structured_value(
            object.get("subject_action").unwrap_or(&Value::Null),
            &format!("视频提示词 temporal_beats[{}].subject_action", index + 1),
        )?;
        validate_structured_value(
            object.get("continuity_target").unwrap_or(&Value::Null),
            &format!("视频提示词 temporal_beats[{}].continuity_target", index + 1),
        )?;
        expected_start = end_sec;
    }

    if (expected_start - f64::from(duration_secs)).abs() > 0.05 {
        return Err(AppError::InvalidArgument(
            "视频提示词 temporal_beats 最后一段 end_sec 必须等于 duration_secs".to_string(),
        ));
    }
    Ok(())
}

fn validate_dialogue_beats(
    value: &Value,
    allowed_source_span_ids: &[String],
    policy: DialogueTracePolicy,
) -> Result<Value, AppError> {
    let beats = value.as_array().ok_or_else(|| {
        AppError::InvalidArgument("视频提示词 dialogue_cues 必须是数组".to_string())
    })?;

    if beats.is_empty() {
        return Ok(Value::Array(Vec::new()));
    }
    if allowed_source_span_ids.is_empty() {
        return Err(AppError::InvalidArgument(
            "视频提示词 dialogue_cues 无法补齐 source_span_ids：当前片段 source_span_ids 为空"
                .to_string(),
        ));
    }

    let derived_source_span_ids = Value::Array(
        allowed_source_span_ids
            .iter()
            .map(|id| Value::String(id.clone()))
            .collect(),
    );
    let mut normalized = Vec::with_capacity(beats.len());
    for (index, beat) in beats.iter().enumerate() {
        let object = beat.as_object().ok_or_else(|| {
            AppError::InvalidArgument(format!(
                "视频提示词 dialogue_cues[{}] 必须是对象",
                index + 1
            ))
        })?;
        validate_dialogue_trace_fields(object, index, allowed_source_span_ids, policy)?;
        for key in object.keys() {
            if !(VIDEO_PROMPT_DIALOGUE_BEAT_FIELDS.contains(&key.as_str())
                || matches!(policy, DialogueTracePolicy::InternalNormalized)
                    && key == "source_span_ids")
            {
                return Err(AppError::InvalidArgument(format!(
                    "视频提示词 dialogue_cues[{}].{} 不是允许字段；dialogue_cues 每条只能包含 {}",
                    index + 1,
                    key,
                    VIDEO_PROMPT_DIALOGUE_BEAT_FIELDS.join("、")
                )));
            }
        }
        for key in VIDEO_PROMPT_DIALOGUE_BEAT_FIELDS {
            validate_structured_value(
                object.get(*key).unwrap_or(&Value::Null),
                &format!("视频提示词 dialogue_cues[{}].{}", index + 1, key),
            )?;
        }
        let mut normalized_object = object.clone();
        normalized_object.insert(
            "source_span_ids".to_string(),
            derived_source_span_ids.clone(),
        );
        normalized.push(Value::Object(normalized_object));
    }
    Ok(Value::Array(normalized))
}

fn validate_dialogue_trace_fields(
    object: &serde_json::Map<String, Value>,
    index: usize,
    allowed_source_span_ids: &[String],
    policy: DialogueTracePolicy,
) -> Result<(), AppError> {
    if object.contains_key("source_span_id") {
        return Err(AppError::InvalidArgument(format!(
            "视频提示词 dialogue_cues[{}] 不要输出 source_span_id/source_span_ids；后端会根据当前片段 source_span_ids 补齐内部溯源",
            index + 1
        )));
    }

    match policy {
        DialogueTracePolicy::ModelOutput => {
            if object.contains_key("source_span_ids") {
                return Err(AppError::InvalidArgument(format!(
                    "视频提示词 dialogue_cues[{}] 不要输出 source_span_id/source_span_ids；后端会根据当前片段 source_span_ids 补齐内部溯源",
                    index + 1
                )));
            }
        }
        DialogueTracePolicy::InternalNormalized => {
            let stored_ids = object
                .get("source_span_ids")
                .ok_or_else(|| {
                    AppError::InvalidArgument(format!(
                        "已保存视频提示词 dialogue_beats[{}] 缺少后端补齐的 source_span_ids",
                        index + 1
                    ))
                })?
                .as_array()
                .ok_or_else(|| {
                    AppError::InvalidArgument(format!(
                        "已保存视频提示词 dialogue_beats[{}].source_span_ids 必须是数组",
                        index + 1
                    ))
                })?;
            let stored_ids = stored_ids
                .iter()
                .map(|value| {
                    value.as_str().map(str::to_string).ok_or_else(|| {
                        AppError::InvalidArgument(format!(
                            "已保存视频提示词 dialogue_beats[{}].source_span_ids 只能包含字符串",
                            index + 1
                        ))
                    })
                })
                .collect::<Result<Vec<_>, _>>()?;
            if stored_ids != allowed_source_span_ids {
                return Err(AppError::InvalidArgument(format!(
                    "已保存视频提示词 dialogue_beats[{}].source_span_ids 与当前片段溯源不一致，请重新生成该视频提示词",
                    index + 1
                )));
            }
        }
    }
    Ok(())
}

fn validate_reference_binding(value: &Value, reference_image_ids: &[i64]) -> Result<(), AppError> {
    let object = value.as_object().ok_or_else(|| {
        AppError::InvalidArgument("视频提示词 reference_binding 必须是对象".to_string())
    })?;
    if object.is_empty() {
        return Err(AppError::InvalidArgument(
            "视频提示词 reference_binding 不能为空".to_string(),
        ));
    }
    validate_structured_value(
        object.get("coverage_check").unwrap_or(&Value::Null),
        "视频提示词 reference_binding.coverage_check",
    )?;

    let bindings = match object.get("bindings") {
        Some(value) => Some(value.as_array().ok_or_else(|| {
            AppError::InvalidArgument(
                "视频提示词 reference_binding.bindings 必须是数组".to_string(),
            )
        })?),
        None => None,
    };
    if reference_image_ids.is_empty() {
        if bindings.is_some_and(|items| !items.is_empty()) {
            return Err(AppError::InvalidArgument(
                "视频提示词 reference_binding.bindings 不能绑定未写入 reference_image_ids 的图片"
                    .to_string(),
            ));
        }
        return Ok(());
    }
    let Some(bindings) = bindings else {
        return Err(AppError::InvalidArgument(
            "视频提示词 reference_binding.bindings 不能为空".to_string(),
        ));
    };
    if bindings.is_empty() {
        return Err(AppError::InvalidArgument(
            "视频提示词 reference_binding.bindings 不能为空".to_string(),
        ));
    }

    let allowed = reference_image_ids.iter().copied().collect::<HashSet<_>>();
    let mut covered = HashSet::new();
    for (index, binding) in bindings.iter().enumerate() {
        let object = binding.as_object().ok_or_else(|| {
            AppError::InvalidArgument(format!(
                "视频提示词 reference_binding.bindings[{}] 必须是对象",
                index + 1
            ))
        })?;
        let image_id = object
            .get("image_id")
            .and_then(Value::as_i64)
            .ok_or_else(|| {
                AppError::InvalidArgument(format!(
                    "视频提示词 reference_binding.bindings[{}].image_id 必须是整数",
                    index + 1
                ))
            })?;
        if image_id <= 0 || !allowed.contains(&image_id) {
            return Err(AppError::InvalidArgument(format!(
                "视频提示词 reference_binding.bindings[{}].image_id 必须来自 reference_image_ids",
                index + 1
            )));
        }
        validate_structured_value(
            object.get("entity").unwrap_or(&Value::Null),
            &format!(
                "视频提示词 reference_binding.bindings[{}].entity",
                index + 1
            ),
        )?;
        validate_structured_value(
            object.get("role").unwrap_or(&Value::Null),
            &format!("视频提示词 reference_binding.bindings[{}].role", index + 1),
        )?;
        validate_structured_value(
            object.get("usage").unwrap_or(&Value::Null),
            &format!("视频提示词 reference_binding.bindings[{}].usage", index + 1),
        )?;
        validate_structured_value(
            object.get("must_keep").unwrap_or(&Value::Null),
            &format!(
                "视频提示词 reference_binding.bindings[{}].must_keep",
                index + 1
            ),
        )?;
        covered.insert(image_id);
    }

    if covered.len() != allowed.len() {
        let mut missing_ids = allowed.difference(&covered).copied().collect::<Vec<_>>();
        missing_ids.sort_unstable();
        return Err(AppError::InvalidArgument(format!(
            "视频提示词 reference_binding.bindings 必须覆盖所有 reference_image_ids，缺少图片 ID：{}",
            missing_ids
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(", ")
        )));
    }
    Ok(())
}

fn validate_duration(duration_secs: i32, field: &str) -> Result<(), AppError> {
    if !(1..=60).contains(&duration_secs) {
        return Err(AppError::InvalidArgument(format!(
            "{field} 必须在 1 到 60 秒之间"
        )));
    }
    Ok(())
}

pub(super) fn validate_reference_ids(ids: &[i64], field: &str) -> Result<(), AppError> {
    let mut seen = HashSet::with_capacity(ids.len());
    for (index, id) in ids.iter().copied().enumerate() {
        if id <= 0 {
            return Err(AppError::InvalidArgument(format!(
                "{field}[{index}] 必须为正整数"
            )));
        }
        if !seen.insert(id) {
            return Err(AppError::InvalidArgument(format!(
                "{field} 包含重复 ID：{id}"
            )));
        }
    }
    Ok(())
}

fn validate_segment_numbers(numbers: &[i32], field: &str) -> Result<(), AppError> {
    if numbers.is_empty() {
        return Err(AppError::InvalidArgument(format!("{field} 不能为空")));
    }
    if numbers.iter().any(|number| *number <= 0) {
        return Err(AppError::InvalidArgument(format!("{field} 必须为正整数")));
    }
    Ok(())
}
