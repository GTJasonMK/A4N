use serde_json::json;

use a4n_error::AppError;

use super::{
    FinalVideoPromptContext, VideoPromptCommonConstraintInput, VideoPromptContractMode,
    final_video_prompt_output_contract, final_video_prompt_public_context, focused_segment_plan,
    insert_rule_value, lightweight_chapter_context, reference_asset_summaries,
    render_strict_json_user_prompt, rule_text, storyboard_frame_for_segment, video_builder_rules,
    video_prompt_common_constraints, video_prompt_task,
};

pub(in crate::video) async fn build_storyboard_video_prompt_user(
    context: FinalVideoPromptContext<'_>,
) -> Result<String, AppError> {
    let FinalVideoPromptContext {
        prompt_renderer,
        project_id,
        chapter_summary,
        chapter_analysis,
        segment_plan,
        frame_prompts: storyboard_frames,
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
    let storyboard_frame = storyboard_frame_for_segment(storyboard_frames, current_segment)?;
    let mut constraints = video_prompt_common_constraints(VideoPromptCommonConstraintInput {
        rules: &rules,
        mode: VideoPromptContractMode::Api,
        style,
        prompt_output_language,
        duration_secs,
        aspect_ratio,
        reference_ids,
    })?;
    insert_rule_value(
        &mut constraints,
        "storyboard_usage_rule",
        &rules,
        "storyboard_video_usage_rule",
    )?;
    insert_rule_value(
        &mut constraints,
        "continuity_rule",
        &rules,
        "storyboard_video_continuity_rule_draft",
    )?;
    let task = video_prompt_task(
        &rules,
        "chapter_video_storyboard_video_prompt",
        "task_instruction_chapter_video_storyboard_video_prompt",
    )?;
    let user_instruction = rule_text(
        &rules,
        "user_instruction_chapter_video_storyboard_video_prompt",
    )?;
    let reference_asset_summaries = reference_asset_summaries(available_assets)?;
    let focused_segment_plan = focused_segment_plan(segment_plan, current_segment)?;
    let payload = final_video_prompt_public_context(&json!({
        "task": task,
        "chapter": lightweight_chapter_context(chapter_summary, chapter_analysis),
        "previous_steps": {
            "focused_chapter_video_segment_plan": focused_segment_plan,
            "focused_chapter_video_storyboard_prompts": storyboard_frame.clone(),
            "previous_completed_shot": previous_completed_shot
        },
        "current_segment": current_segment,
        "storyboard_frame": storyboard_frame,
        "reference_asset_summaries": reference_asset_summaries,
        "constraints": constraints,
        "output_contract": final_video_prompt_output_contract("chapter_video_storyboard_video_prompt")
    }));
    render_strict_json_user_prompt(prompt_renderer, project_id, &user_instruction, &payload).await
}
