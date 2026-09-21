use serde_json::json;

use crate::video::prompt_generation::frame_prompts::{
    AppError, FinalVideoPromptContext, ProgressSpan, VideoPromptChunk, VideoPromptProgress,
    ensure_video_prompt_generation_can_start, report_video_prompt_progress,
};

use super::draft::generate_video_prompt_draft;
use super::{
    ExecutedVideoPromptUnit, VideoPromptUnitCheckpointContext, VideoPromptUnitExecutionInput,
};

pub(in crate::video::prompt_generation::frame_prompts::video_chain) async fn execute_video_prompt_unit(
    input: VideoPromptUnitExecutionInput<'_, '_>,
    progress: &mut Option<VideoPromptProgress<'_>>,
    chunks: &mut Option<VideoPromptChunk<'_>>,
) -> Result<ExecutedVideoPromptUnit, AppError> {
    let VideoPromptUnitExecutionInput {
        service,
        spec,
        project_id,
        chapter_id,
        source,
        prompt_renderer,
        vars,
        workflow_run_id,
        cancel_token,
        segment,
        segment_by_number,
        video_segment_json,
        frame_json,
        previous_completed_shot,
        available_assets,
        style,
        prompt_output_language,
        duration_secs,
        aspect_ratio,
        reference_ids,
        allowed_reference_ids,
        is_chat_video,
        segment_index,
        current_index,
        total_segments,
        generation_number,
        generation_limit,
    } = input;
    let shot_number = segment.segment_number;
    ensure_video_prompt_generation_can_start(
        service,
        project_id,
        chapter_id,
        spec.workflow_mode,
        shot_number,
        spec.video_prompt_label,
        prompt_output_language,
    )
    .await?;
    let current_segment_json = json!(segment);
    let previous_shot_json = previous_completed_shot
        .map(|shot| shot.to_json_value())
        .transpose()?;
    let generation_message = format!(
        "正在生成{}全章第 {} 段（共 {} 段），本批第 {}/{} 条",
        spec.video_prompt_label, current_index, total_segments, generation_number, generation_limit
    );
    let progress_span = ProgressSpan::new(60, 90).split(segment_index, total_segments);
    let checkpoint_context = VideoPromptUnitCheckpointContext {
        service,
        spec,
        project_id,
        chapter_id,
        source,
        prompt_renderer,
        vars,
        workflow_run_id,
        cancel_token,
        segment_by_number,
        final_prompt_context: FinalVideoPromptContext {
            prompt_renderer,
            project_id,
            chapter_summary: source.summary,
            chapter_analysis: source.analysis,
            segment_plan: video_segment_json,
            frame_prompts: frame_json,
            current_segment: &current_segment_json,
            previous_completed_shot: previous_shot_json.as_ref(),
            available_assets,
            style,
            prompt_output_language,
            duration_secs,
            aspect_ratio,
            reference_ids,
        },
        shot_number,
        prompt_output_language,
        allowed_reference_ids,
        is_chat_video,
    };
    let executed = generate_video_prompt_draft(
        checkpoint_context,
        progress_span,
        &generation_message,
        progress,
        chunks,
    )
    .await?;
    report_video_prompt_progress(
        progress,
        progress_span.end(),
        100,
        &format!(
            "{}第 {}/{} 段已生成，等待整体验证后保存，本批已生成 {} 条",
            spec.video_prompt_label, current_index, total_segments, generation_number
        ),
    )?;
    Ok(executed)
}
