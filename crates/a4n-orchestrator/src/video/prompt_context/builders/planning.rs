use serde_json::{Value, json};

use a4n_core::error::AppError;

use a4n_core::source_spans::{SourceSpan, source_span_prompt_values};

use super::{
    VideoPlanningPromptContext, chapter_analysis_context, render_strict_json_user_prompt,
    rule_text, rule_value, video_builder_rules, video_prompt_task,
};

pub(in crate::video) async fn build_segment_plan_for_shot_user(
    context: VideoPlanningPromptContext<'_>,
    current_shot: &Value,
    source_spans: &[SourceSpan],
) -> Result<String, AppError> {
    let rules = video_builder_rules(context.prompt_renderer, context.project_id).await?;
    let task = video_prompt_task(
        &rules,
        "chapter_video_segment_plan",
        "task_instruction_chapter_video_segment_plan",
    )?;
    let user_instruction = rule_text(&rules, "user_instruction_chapter_video_segment_plan")?;
    let payload = json!({
        "task": task,
        "chapter": {
            "source_spans": source_span_prompt_values(source_spans),
            "summary": context.chapter_summary.map(str::trim).filter(|s| !s.is_empty()),
            "analysis": chapter_analysis_context(context.chapter_analysis)
        },
        "project_context": context.project_context,
        "current_shot": current_shot,
        "constraints": {
            "style": context.style,
            "selected_duration_secs": context.duration_secs,
            "aspect_ratio": context.aspect_ratio,
            "style_preservation_rule": rule_value(&rules, "style_preservation_rule")?,
            "time_budget_rule": rule_value(&rules, "time_budget_rule")?,
            "safe_visual_rewrite_rule": rule_value(&rules, "safe_visual_rewrite_rule")?,
            "source_span_rule": rule_value(&rules, "segment_plan_source_span_rule")?,
            "shooting_script_projection_rule": rule_value(&rules, "segment_plan_shooting_script_projection_rule")?,
            "beat_mapping_rule": rule_value(&rules, "segment_plan_beat_mapping_rule")?,
            "shot_mapping_rule": rule_value(&rules, "segment_plan_shot_mapping_rule")?,
            "duration_selection_rule": rule_value(&rules, "segment_plan_duration_selection_rule")?
        },
        "output_contract": {
            "format": "json_object",
            "prompt_name": "chapter_video_segment_plan",
            "required_root_field": "segments",
            "required_segment_fields": [
                "source_shot_id",
                "covered_beat_ids",
                "title",
                "scene_summary",
                "blocking_summary",
                "spatial_anchors",
                "emotion_action_goals",
                "video_intent",
                "emotional_arc",
                "performance_plan",
                "material_coverage"
            ],
            "derived_segment_fields": [
                "segment_number",
                "segment_index_in_shot",
                "shot_segment_count",
                "source_span_ids",
                "duration_secs",
                "aspect_ratio"
            ],
            "segment_number_rule": "只为 current_shot 生成 1 个或多个连续 segments；source_shot_id 必须等于 current_shot.source_shot_id；covered_beat_ids 必须使用 current_shot.action_beats[].beat_id 的稳定字符串 ID。不要输出 segment_number、segment_index_in_shot、shot_segment_count、source_span_ids、duration_secs、aspect_ratio 或 reference_image_ids；后端会按数组顺序、拍摄剧本投影、用户选择时长和用户画幅推导结构字段。片段规划只做素材需求审计，不绑定具体参考图。",
            "performance_action_script_rule": "performance_plan.action_script 每个对象必须同时包含 character、body、movement、hands、gaze、timing 六个字段，且全部必须是非空字符串。hands 没有明显手部动作时也必须写可见静止状态或未接触道具状态，例如“手自然垂在身侧”“手未接触道具”“双手保持稳定”；禁止空字符串、null、对象、数组或省略字段。"
        }
    });
    render_strict_json_user_prompt(
        context.prompt_renderer,
        context.project_id,
        &user_instruction,
        &payload,
    )
    .await
}
