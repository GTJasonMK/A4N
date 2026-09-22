use a4n_db::workflow::UpsertWorkflowCheckpointInput;
use a4n_domain::workflow::WORKFLOW_STATUS_COMPLETED;
use a4n_error::AppError;
use serde_json::json;

use crate::galgame::{
    GalgamePipelineStartStage, GalgameScriptDetail, build_galgame_script_persist_plan,
};

use super::super::super::contracts::{
    CHECKPOINT_LINES_STAGED, CHECKPOINT_PERSIST_SCRIPT, CHECKPOINT_SCENES, CHECKPOINT_VISUAL_PLAN,
};
use super::super::super::progress::ensure_galgame_pipeline_not_cancelled;
use super::super::contracts::{
    GalgamePipelineEarlyOutputs, GalgamePipelineExecution, GalgamePipelineLateOutputs,
};

pub(in crate::galgame::pipeline_run::pipeline) async fn persist_galgame_pipeline_script(
    execution: &GalgamePipelineExecution<'_>,
    early: &GalgamePipelineEarlyOutputs,
    late: &GalgamePipelineLateOutputs,
) -> Result<GalgameScriptDetail, AppError> {
    let stage = GalgamePipelineStartStage::PersistScript;
    ensure_galgame_pipeline_not_cancelled(execution.service, execution.options.workflow_run_id)
        .await?;
    execution
        .service
        .mark_workflow_current_step(execution.options.workflow_run_id, stage.key())
        .await?;
    execution
        .service
        .report_pipeline_progress(
            execution.options.workflow_run_id,
            execution.options.task_id.as_deref(),
            execution.options.progress_reporter.as_ref(),
            stage,
            6,
            stage.message(),
        )
        .await?;

    let persist_plan = build_galgame_script_persist_plan(
        &early.scenes,
        &early.staging,
        &late.visual_plan,
        &early.branch_plan,
        &execution.prompt_context.source_spans,
    )?;
    let detail = execution
        .service
        .persist_pipeline_script(execution.project_id, persist_plan)
        .await?;
    let line_count = detail
        .scenes
        .iter()
        .map(|scene| scene.lines.len())
        .sum::<usize>();
    execution
        .service
        .upsert_workflow_checkpoint(UpsertWorkflowCheckpointInput {
            workflow_run_id: execution.options.workflow_run_id,
            step_key: CHECKPOINT_PERSIST_SCRIPT.to_string(),
            step_order: stage.order(),
            status: WORKFLOW_STATUS_COMPLETED.to_string(),
            input_fingerprint_sha256: None,
            input_fingerprint_json: Some(json!({
                "project_id": execution.project_id,
                "source_snapshot_id": execution.prompt_context.source_snapshot_id,
                "scenes_checkpoint": CHECKPOINT_SCENES,
                "lines_staged_checkpoint": CHECKPOINT_LINES_STAGED,
                "visual_plan_checkpoint": CHECKPOINT_VISUAL_PLAN,
            })),
            checkpoint_json: Some(json!({
                "script_id": detail.id,
                "scene_count": detail.scenes.len(),
                "line_count": line_count,
                "branch_choice_count": early.branch_plan.choices.len(),
                "branch_design_persisted": !early.branch_plan.choices.is_empty(),
            })),
            artifacts_json: Some(json!({
                "galgame_script_id": detail.id,
            })),
            generation_run_ids_json: None,
            error_message: None,
            completed: true,
        })
        .await?;
    execution
        .service
        .report_pipeline_progress(
            execution.options.workflow_run_id,
            execution.options.task_id.as_deref(),
            execution.options.progress_reporter.as_ref(),
            stage,
            6,
            "已写入 Galgame 剧本",
        )
        .await?;
    Ok(detail)
}
