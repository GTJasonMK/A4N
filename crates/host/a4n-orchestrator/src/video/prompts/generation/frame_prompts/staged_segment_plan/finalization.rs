use a4n_error::AppError;
use serde_json::json;

use super::super::{
    parse_video_segment_plan_response, save_completed_video_workflow_checkpoint,
    validate_segment_plan_against_shooting_script_projection,
};
use super::contracts::{FinalizeStagedSegmentPlanInput, StagedSegmentPlanResult};
use super::exclude_reused_segment_plan_shots;

pub(super) async fn finalize_staged_segment_plan(
    input: FinalizeStagedSegmentPlanInput<'_>,
) -> Result<StagedSegmentPlanResult, AppError> {
    let FinalizeStagedSegmentPlanInput {
        service,
        workflow_run_id,
        source_spans,
        shooting_script_projection,
        cancel_token,
        rendered_shots,
    } = input;
    let combined_raw = json!({ "segments": rendered_shots.segment_inputs }).to_string();
    let segment_plan =
        parse_video_segment_plan_response(&combined_raw, source_spans, shooting_script_projection)?;
    validate_segment_plan_against_shooting_script_projection(
        &segment_plan,
        shooting_script_projection,
    )?;
    if cancel_token.is_some_and(tokio_util::sync::CancellationToken::is_cancelled) {
        return Err(AppError::Cancelled);
    }
    let stage_outputs = json!({ "segment_plan_per_shot": rendered_shots.stage_outputs });
    // final 仅冻结草稿编辑；恢复始终逐镜头核对当前模板与来源，不再存一份可绕过校验的聚合结果。
    save_completed_video_workflow_checkpoint(
        service,
        workflow_run_id,
        "video_segment_plan.final".to_string(),
        299,
        json!({
            "schema_version": 3,
            "segment_plan_sha256": a4n_domain::source_text_hash(&serde_json::to_string(&segment_plan)?),
        }),
        json!({"segment_count": segment_plan.len()}),
        None,
    )
    .await?;
    Ok((
        rendered_shots.last_trace,
        exclude_reused_segment_plan_shots(segment_plan, &rendered_shots.reused_source_shot_ids),
        stage_outputs,
    ))
}
