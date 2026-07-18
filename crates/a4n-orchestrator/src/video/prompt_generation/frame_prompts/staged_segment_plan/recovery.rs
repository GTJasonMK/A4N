use a4n_core::error::AppError;

use super::super::{
    VideoSegmentPlan, checkpoint_field, load_completed_video_workflow_checkpoint,
    report_video_prompt_progress, validate_segment_plan_against_shooting_script_projection,
};
use super::contracts::{CompletedSegmentPlanRecoveryInput, StagedSegmentPlanResult};

pub(super) async fn reuse_completed_staged_segment_plan(
    input: CompletedSegmentPlanRecoveryInput<'_, '_>,
) -> Result<Option<StagedSegmentPlanResult>, AppError> {
    let CompletedSegmentPlanRecoveryInput {
        service,
        workflow_run_id,
        fingerprint_json,
        shooting_script_projection,
        progress_message,
        progress_span,
        progress,
    } = input;
    let Some(checkpoint) = load_completed_video_workflow_checkpoint(
        service,
        workflow_run_id,
        "video_segment_plan.final",
        fingerprint_json,
    )
    .await?
    else {
        return Ok(None);
    };
    let segment_json = checkpoint_field(&checkpoint, "segment_json", "视频片段规划 final 断点")?;
    let segment_plan: Vec<VideoSegmentPlan> =
        checkpoint_field(&checkpoint, "segment_plan", "视频片段规划 final 断点")?;
    let stage_outputs = checkpoint_field(&checkpoint, "stage_outputs", "视频片段规划 final 断点")?;
    validate_segment_plan_against_shooting_script_projection(
        &segment_plan,
        shooting_script_projection,
    )?;
    report_video_prompt_progress(
        progress,
        progress_span.end(),
        100,
        &format!("{progress_message}（已复用断点）"),
    )?;
    Ok(Some((None, segment_json, segment_plan, stage_outputs)))
}
