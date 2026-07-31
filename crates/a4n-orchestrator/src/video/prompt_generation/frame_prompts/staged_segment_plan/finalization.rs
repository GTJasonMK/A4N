use a4n_core::error::AppError;
use serde_json::json;

use super::super::{
    parse_generated_segment_plan_with_current_config, save_completed_video_workflow_checkpoint,
    validate_segment_plan_against_shooting_script_projection,
};
use super::contracts::{FinalizeStagedSegmentPlanInput, StagedSegmentPlanResult};

pub(super) async fn finalize_staged_segment_plan(
    input: FinalizeStagedSegmentPlanInput<'_>,
) -> Result<StagedSegmentPlanResult, AppError> {
    let FinalizeStagedSegmentPlanInput {
        service,
        workflow_run_id,
        fingerprint_json,
        source_spans,
        shooting_script_projection,
        duration_secs,
        default_aspect_ratio,
        cancel_token,
        rendered_shots,
    } = input;
    let combined_raw = json!({ "segments": rendered_shots.segment_inputs }).to_string();
    let (segment_json, segment_plan) = parse_generated_segment_plan_with_current_config(
        &combined_raw,
        source_spans,
        shooting_script_projection,
        duration_secs,
        default_aspect_ratio,
    )?;
    validate_segment_plan_against_shooting_script_projection(
        &segment_plan,
        shooting_script_projection,
    )?;
    if cancel_token.is_some_and(tokio_util::sync::CancellationToken::is_cancelled) {
        return Err(AppError::Cancelled);
    }
    let stage_outputs = json!({ "segment_plan_per_shot": rendered_shots.stage_outputs });
    save_completed_video_workflow_checkpoint(
        service,
        workflow_run_id,
        "video_segment_plan.final".to_string(),
        299,
        fingerprint_json,
        json!({
            "segment_json": segment_json.clone(),
            "segment_plan": segment_plan.clone(),
            "stage_outputs": stage_outputs.clone(),
        }),
        None,
    )
    .await?;
    Ok((
        rendered_shots.last_trace,
        segment_json,
        segment_plan,
        stage_outputs,
    ))
}
