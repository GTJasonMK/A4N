use serde_json::{Value, json};

use crate::video::prompts::json::{PromptTextKind, prompt_density_rule};
use a4n_domain::prompt_language::PromptOutputLanguage;
use a4n_error::AppError;

use super::{
    FramePromptContext, chapter_analysis_context, frame_prompt_context,
    frame_reference_slot_order_rule, frame_reference_slots, lightweight_chapter_context,
    render_strict_json_user_prompt, required_current_frame_field, rule_text, rule_value,
    video_builder_rules, video_prompt_task,
};

pub(in crate::video) async fn build_single_frame_prompt_user(
    context: FramePromptContext<'_>,
    current_frame: &Value,
) -> Result<String, AppError> {
    let FramePromptContext {
        prompt_renderer,
        project_id,
        chapter_summary,
        chapter_analysis,
        segment_plan,
        available_assets,
        style,
        prompt_output_language,
        aspect_ratio,
        reference_ids,
    } = context;
    let rules = video_builder_rules(prompt_renderer, project_id).await?;
    let required_frame_number =
        required_current_frame_field(current_frame, "frame_number", "integer")?;
    let required_segment_numbers =
        required_current_frame_field(current_frame, "expected_segment_numbers", "array")?;
    let required_duration_secs =
        required_current_frame_field(current_frame, "required_duration_secs", "integer")?;
    let required_aspect_ratio =
        required_current_frame_field(current_frame, "required_aspect_ratio", "string")?;
    let task = video_prompt_task(
        &rules,
        "chapter_video_frame_prompts",
        "task_instruction_chapter_video_frame_prompts",
    )?;
    let first_slot_role = current_frame
        .get("previous_frame_image")
        .filter(|value| !value.is_null())
        .map(|_| "previous_boundary_frame");
    let reference_slots = frame_reference_slots(reference_ids, first_slot_role)?;
    let user_instruction = rule_text(&rules, "user_instruction_chapter_video_frame_prompts")?;
    let payload = json!({
        "task": task,
        "chapter": lightweight_chapter_context(chapter_summary, chapter_analysis),
        "previous_steps": {
            "focused_chapter_video_segment_plan": frame_prompt_context(segment_plan, current_frame)
        },
        "current_frame": current_frame,
        "available_assets": available_assets,
        "constraints": {
            "style": style,
            "prompt_output_language": prompt_output_language.to_prompt_json(),
            "frame_prompt_required_sections": prompt_output_language.frame_sections(),
            "aspect_ratio": aspect_ratio,
            "reference_slots": reference_slots,
            "single_frame_rule": rule_value(&rules, "single_frame_rule")?,
            "style_preservation_rule": rule_value(&rules, "single_frame_style_preservation_rule")?,
            "frame_prompt_section_rule": rule_value(&rules, "frame_image_prompt_section_rule")?,
            "boundary_continuity_rule": rule_value(&rules, "frame_boundary_continuity_rule")?,
            "previous_frame_rule": rule_value(&rules, "single_frame_previous_frame_rule")?,
            "reference_display_label_rule": rule_value(&rules, "reference_display_label_rule")?,
            "reference_slot_order_rule": frame_reference_slot_order_rule(),
            "segment_directing_rule": rule_value(&rules, "single_frame_segment_directing_rule")?,
            "material_coverage_rule": rule_value(&rules, "material_coverage_rule")?,
            "prompt_density_rule": frame_prompt_density_rule(prompt_output_language)
        },
        "output_contract": {
            "format": "json_object",
            "prompt_name": "chapter_video_frame_prompts",
            "required_root_field": "frames",
            "required_frame_count": 1,
            "required_frame_number": required_frame_number,
            "required_segment_numbers": required_segment_numbers,
            "required_duration_secs": required_duration_secs,
            "required_aspect_ratio": required_aspect_ratio
        }
    });
    render_strict_json_user_prompt(prompt_renderer, project_id, &user_instruction, &payload).await
}

pub(in crate::video) async fn build_storyboard_prompt_user(
    context: FramePromptContext<'_>,
    current_frame: &Value,
) -> Result<String, AppError> {
    let FramePromptContext {
        prompt_renderer,
        project_id,
        chapter_summary,
        chapter_analysis,
        segment_plan,
        available_assets,
        style,
        prompt_output_language,
        aspect_ratio,
        reference_ids,
    } = context;
    let rules = video_builder_rules(prompt_renderer, project_id).await?;
    let required_frame_number =
        required_current_frame_field(current_frame, "frame_number", "integer")?;
    let required_segment_numbers =
        required_current_frame_field(current_frame, "expected_segment_numbers", "array")?;
    let required_duration_secs =
        required_current_frame_field(current_frame, "required_duration_secs", "integer")?;
    let required_aspect_ratio =
        required_current_frame_field(current_frame, "required_aspect_ratio", "string")?;
    let task = video_prompt_task(
        &rules,
        "chapter_video_storyboard_prompts",
        "task_instruction_chapter_video_storyboard_prompts",
    )?;
    let reference_slots = frame_reference_slots(reference_ids, None)?;
    let user_instruction = rule_text(&rules, "user_instruction_chapter_video_storyboard_prompts")?;
    let payload = json!({
        "task": task,
        "chapter": lightweight_chapter_context(chapter_summary, chapter_analysis),
        "previous_steps": {
            "focused_chapter_video_segment_plan": frame_prompt_context(segment_plan, current_frame)
        },
        "current_storyboard_frame": current_frame,
        "available_assets": available_assets,
        "constraints": {
            "style": style,
            "prompt_output_language": prompt_output_language.to_prompt_json(),
            "frame_prompt_required_sections": prompt_output_language.frame_sections(),
            "aspect_ratio": aspect_ratio,
            "reference_slots": reference_slots,
            "storyboard_frame_rule": rule_value(&rules, "storyboard_frame_rule")?,
            "storyboard_panel_count_rule": rule_value(&rules, "storyboard_panel_count_rule")?,
            "style_preservation_rule": rule_value(&rules, "style_preservation_rule")?,
            "reference_display_label_rule": rule_value(&rules, "reference_display_label_rule")?,
            "reference_slot_order_rule": frame_reference_slot_order_rule(),
            "segment_directing_rule": rule_value(&rules, "storyboard_segment_directing_rule")?,
            "material_coverage_rule": rule_value(&rules, "material_coverage_rule")?,
            "prompt_density_rule": frame_prompt_density_rule(prompt_output_language)
        },
        "output_contract": {
            "format": "json_object",
            "prompt_name": "chapter_video_storyboard_prompts",
            "required_root_field": "frames",
            "required_frame_count": 1,
            "required_frame_number": required_frame_number,
            "required_segment_numbers": required_segment_numbers,
            "required_duration_secs": required_duration_secs,
            "required_aspect_ratio": required_aspect_ratio,
            "required_storyboard_panel_count_range": [2, 6],
            "semantics": "这一条 frame 是当前视频单元的一张多格分镜板图片，不是片段首帧或尾帧；storyboard_panels 描述分镜板内部画格。"
        }
    });
    render_strict_json_user_prompt(prompt_renderer, project_id, &user_instruction, &payload).await
}

pub(in crate::video) async fn build_first_frame_prompt_user(
    context: FramePromptContext<'_>,
) -> Result<String, AppError> {
    let FramePromptContext {
        prompt_renderer,
        project_id,
        chapter_summary,
        chapter_analysis,
        segment_plan,
        available_assets,
        style,
        prompt_output_language,
        aspect_ratio,
        reference_ids,
    } = context;
    let rules = video_builder_rules(prompt_renderer, project_id).await?;
    let task = video_prompt_task(
        &rules,
        "chapter_video_first_frame_prompts",
        "task_instruction_chapter_video_first_frame_prompts",
    )?;
    let reference_slots = frame_reference_slots(reference_ids, None)?;
    let user_instruction = rule_text(&rules, "user_instruction_chapter_video_first_frame_prompts")?;
    let first_segment = segment_plan
        .get("segments")
        .and_then(Value::as_array)
        .and_then(|segments| segments.first())
        .ok_or_else(|| AppError::InvalidArgument("首帧生成缺少首个视频片段".to_string()))?;
    let payload = json!({
        "task": task,
        "chapter": {
            "summary": chapter_summary.map(str::trim).filter(|s| !s.is_empty()),
            "analysis": chapter_analysis_context(chapter_analysis)
        },
        "previous_steps": {
            "chapter_video_segment_plan": {"segments": [first_segment]}
        },
        "available_assets": available_assets,
        "constraints": {
            "style": style,
            "prompt_output_language": prompt_output_language.to_prompt_json(),
            "frame_prompt_required_sections": prompt_output_language.frame_sections(),
            "aspect_ratio": aspect_ratio,
            "reference_slots": reference_slots,
            "style_preservation_rule": rule_value(&rules, "single_frame_style_preservation_rule")?,
            "initial_frame_rule": rule_value(&rules, "initial_frame_rule")?,
            "frame_scope_rule": rule_value(&rules, "initial_frame_scope_rule")?,
            "frame_prompt_section_rule": rule_value(&rules, "frame_image_prompt_section_rule")?,
            "segment_directing_rule": rule_value(&rules, "initial_frame_segment_directing_rule")?,
            "material_coverage_rule": rule_value(&rules, "material_coverage_rule")?,
            "reference_slot_order_rule": frame_reference_slot_order_rule(),
            "prompt_density_rule": frame_prompt_density_rule(prompt_output_language),
            "image_input_semantics_rule": rule_value(&rules, "initial_frame_image_input_semantics_rule")?
        },
        "output_contract": {
            "format": "json_object",
            "prompt_name": "chapter_video_first_frame_prompts",
            "required_root_field": "frames",
            "required_frame_fields": [
                "title",
                "scene_summary",
                "frame_prompt",
                "blocking_state",
                "lighting_state",
                "negative_prompt"
            ],
            "derived_frame_fields": [
                "frame_number",
                "duration_secs",
                "aspect_ratio",
                "segment_numbers"
            ],
            "frame_number_rule": "frames 数组只能包含 1 条记录；不要输出 frame_number、duration_secs、aspect_ratio、segment_numbers 或 reference_image_ids，后端会按第 1 个片段和用户选择补齐。"
        }
    });
    render_strict_json_user_prompt(prompt_renderer, project_id, &user_instruction, &payload).await
}

fn frame_prompt_density_rule(language: PromptOutputLanguage) -> String {
    format!(
        "{}；{}",
        prompt_density_rule(language, PromptTextKind::FramePrompt, "frame_prompt"),
        prompt_density_rule(language, PromptTextKind::FrameNegative, "negative_prompt"),
    )
}
