use serde_json::json;

use a4n_core::error::AppError;

use super::{
    FinalVideoPromptContext, VideoPromptCommonConstraintInput, VideoPromptContractMode,
    add_input_frame_usage_constraint, final_video_prompt_public_context, focused_segment_plan,
    insert_rule_value, lightweight_chapter_context, reference_asset_summaries,
    render_strict_json_user_prompt, rule_text, video_builder_rules,
    video_prompt_common_constraints, video_prompt_input_frames, video_prompt_task,
};

pub(in crate::video) async fn build_chat_video_prompt_user(
    context: FinalVideoPromptContext<'_>,
) -> Result<String, AppError> {
    let FinalVideoPromptContext {
        prompt_renderer,
        project_id,
        chapter_summary,
        chapter_analysis,
        segment_plan,
        frame_prompts,
        current_segment,
        previous_completed_shot,
        available_assets,
        style,
        prompt_output_language,
        duration_secs,
        aspect_ratio,
        reference_ids,
    } = context;
    let rules = video_builder_rules(prompt_renderer, project_id).await?;
    let input_frames = video_prompt_input_frames(frame_prompts, current_segment)?;
    let mut constraints = video_prompt_common_constraints(VideoPromptCommonConstraintInput {
        rules: &rules,
        mode: VideoPromptContractMode::Chat,
        style,
        prompt_output_language,
        duration_secs,
        aspect_ratio,
        reference_ids,
    })?;
    add_input_frame_usage_constraint(
        &mut constraints,
        &input_frames,
        previous_completed_shot,
        prompt_output_language,
    );
    insert_rule_value(
        &mut constraints,
        "chat_prompt_scope",
        &rules,
        "chat_prompt_scope_rule",
    )?;
    constraints.insert(
        "required_sections".to_string(),
        json!(prompt_output_language.visual_sections()),
    );
    insert_rule_value(
        &mut constraints,
        "input_image_rule",
        &rules,
        "chat_input_image_rule_draft",
    )?;
    insert_rule_value(
        &mut constraints,
        "director_rule",
        &rules,
        "chat_director_rule",
    )?;
    insert_rule_value(
        &mut constraints,
        "continuity_rule",
        &rules,
        "chat_continuity_rule_draft",
    )?;
    let task = video_prompt_task(
        &rules,
        "chapter_video_chat_prompt",
        "task_instruction_chapter_video_chat_prompt",
    )?;
    let user_instruction = rule_text(&rules, "user_instruction_chapter_video_chat_prompt")?;
    let reference_asset_summaries = reference_asset_summaries(available_assets)?;
    let focused_segment_plan = focused_segment_plan(segment_plan, current_segment)?;
    let payload = final_video_prompt_public_context(&json!({
        "task": task,
        "chapter": lightweight_chapter_context(chapter_summary, chapter_analysis),
        "previous_steps": {
            "focused_chapter_video_segment_plan": focused_segment_plan,
            "focused_chapter_video_frame_prompts": input_frames.clone(),
            "previous_completed_shot": previous_completed_shot
        },
        "current_segment": current_segment,
        "reference_asset_summaries": reference_asset_summaries,
        "input_frames": input_frames,
        "constraints": constraints,
        "output_contract": {
            "format": "json_object",
            "prompt_name": "chapter_video_chat_prompt",
            "required_root_field": "shots",
            "required_shot_count": 1,
            "required_shot_fields": [
                "title",
                "scene_summary",
                "chat_prompt",
                "negative_prompt"
            ],
            "chat_prompt_required_sections": prompt_output_language.visual_sections()
        }
    }));
    render_strict_json_user_prompt(prompt_renderer, project_id, &user_instruction, &payload).await
}
