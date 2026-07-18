mod contracts;
mod finalization;
mod recovery;
mod shots;

use a4n_core::error::AppError;
use a4n_db::workflow;

use crate::workflow::workflow_checkpoint_contract_app_error;

pub(super) use self::contracts::StagedSegmentPlanInput;
use self::contracts::{
    CompletedSegmentPlanRecoveryInput, FinalizeStagedSegmentPlanInput, StagedSegmentPlanResult,
};
use self::finalization::finalize_staged_segment_plan;
use self::recovery::reuse_completed_staged_segment_plan;
use self::shots::render_segment_plan_shots;
use super::video_final_segment_plan_fingerprint;

pub(super) async fn render_staged_segment_plan_with_duration(
    input: StagedSegmentPlanInput<'_, '_, '_, '_>,
) -> Result<StagedSegmentPlanResult, AppError> {
    let workflow_run_id =
        workflow::required_llm_call_workflow_run_id(input.workflow_run_id, "video_segment_plan")
            .map_err(workflow_checkpoint_contract_app_error)?;
    if input.shooting_script_projection.shot_count() == 0 {
        return Err(AppError::InvalidArgument(
            "视频片段规划必须基于至少一个导演镜头".to_string(),
        ));
    }
    let final_fingerprint_json = video_final_segment_plan_fingerprint(
        input.project_id,
        input.source,
        input.shooting_script_projection_json,
        input.style,
        input.duration_secs,
        input.default_aspect_ratio,
    );
    if let Some(result) = reuse_completed_staged_segment_plan(CompletedSegmentPlanRecoveryInput {
        service: input.service,
        workflow_run_id,
        fingerprint_json: &final_fingerprint_json,
        shooting_script_projection: input.shooting_script_projection,
        progress_message: input.progress_message,
        progress_span: input.progress_span,
        progress: &mut *input.progress,
    })
    .await?
    {
        return Ok(result);
    }

    let service = input.service;
    let source_spans = input.source_spans;
    let shooting_script_projection = input.shooting_script_projection;
    let duration_secs = input.duration_secs;
    let default_aspect_ratio = input.default_aspect_ratio;
    let rendered_shots = render_segment_plan_shots(input, workflow_run_id).await?;
    finalize_staged_segment_plan(FinalizeStagedSegmentPlanInput {
        service,
        workflow_run_id,
        fingerprint_json: final_fingerprint_json,
        source_spans,
        shooting_script_projection,
        duration_secs,
        default_aspect_ratio,
        rendered_shots,
    })
    .await
}
