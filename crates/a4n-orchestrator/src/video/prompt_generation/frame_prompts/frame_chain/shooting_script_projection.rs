use crate::video::prompt_generation::frame_prompts::{
    PreparedSegmentPlan, StagedSegmentPlanInput, VideoPromptChunk, VideoPromptProgress,
    render_staged_segment_plan_with_duration, report_video_prompt_progress,
    video_execution_chain_identity, video_projection_from_shooting_script,
    video_prompt_generation_trace,
};
use a4n_core::error::AppError;
use a4n_core::video_execution::VideoShootingScriptIdentityV1;

use super::contracts::{
    SegmentPlanFromShootingScriptProjectionInput, ShootingScriptProjectionPreparationInput,
};

pub(super) async fn render_segment_plan_from_shooting_script_projection(
    input: SegmentPlanFromShootingScriptProjectionInput<'_, '_>,
    progress: &mut Option<VideoPromptProgress<'_>>,
    chunks: &mut Option<VideoPromptChunk<'_>>,
) -> Result<PreparedSegmentPlan, AppError> {
    let SegmentPlanFromShootingScriptProjectionInput {
        shooting_script_projection,
        segment_context,
        segment_progress,
        segment_span,
    } = input;
    let ShootingScriptProjectionPreparationInput {
        service,
        project_id,
        chapter_id,
        source,
        source_spans,
        project_context,
        style,
        duration_secs,
        aspect_ratio,
        prompt_renderer,
        variables,
        workflow_run_id,
        progress_span: shooting_script_projection_span,
        cancel_token,
        ..
    } = shooting_script_projection;
    let resolved = service
        .require_current_chapter_shooting_script(
            crate::shooting_script::ChapterShootingScriptSourceInput {
                project_id,
                source_chapter_id: chapter_id,
            },
        )
        .await?;
    let record = &resolved.shooting_script.record;
    let analysis = video_projection_from_shooting_script(
        &resolved.shooting_script.script,
        VideoShootingScriptIdentityV1 {
            record_id: record.id,
            revision: record.revision,
            source_revision_key: record.source_revision_key.clone(),
            source_content_sha256: record.source_content_sha256.clone(),
            script_sha256: record.script_sha256.clone(),
        },
        duration_secs,
    )?;
    let analysis_json = serde_json::to_value(&analysis).map_err(|error| {
        AppError::InvalidArgument(format!("拍摄剧本视频执行投影序列化失败: {error}"))
    })?;
    report_video_prompt_progress(
        progress,
        shooting_script_projection_span.end(),
        100,
        "已从结构化拍摄剧本构建视频执行投影",
    )?;
    let (segment_trace, segment_json, segment_plan, segment_stage_outputs) =
        render_staged_segment_plan_with_duration(StagedSegmentPlanInput {
            service,
            prompt_renderer,
            project_id,
            workflow_run_id,
            variables,
            source,
            source_spans,
            project_context,
            shooting_script_projection_json: &analysis_json,
            shooting_script_projection: &analysis,
            default_aspect_ratio: aspect_ratio,
            style,
            duration_secs,
            context: segment_context,
            progress_message: segment_progress,
            progress_span: segment_span,
            cancel_token,
            progress,
            chunks,
        })
        .await?;
    Ok(PreparedSegmentPlan {
        segment_stage_outputs,
        segment_trace: segment_trace
            .map(|trace| video_prompt_generation_trace(trace, workflow_run_id)),
        segment_json,
        execution_identity: video_execution_chain_identity(
            &analysis,
            &segment_plan,
            style,
            duration_secs,
            aspect_ratio,
        )?,
        segment_plan,
    })
}
