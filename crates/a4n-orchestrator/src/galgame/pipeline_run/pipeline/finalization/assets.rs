use a4n_core::error::AppError;
use a4n_db::workflow::{
    UpsertWorkflowCheckpointInput, WORKFLOW_STATUS_COMPLETED, mark_workflow_current_step,
    upsert_workflow_checkpoint,
};
use serde_json::json;

use crate::galgame::{GalgamePipelineStartStage, GalgameScriptDetail};
use crate::workflow::workflow_write_app_error;

use super::super::super::contracts::CHECKPOINT_ASSET_GENERATION;
use super::super::super::progress::ensure_galgame_pipeline_not_cancelled;
use super::super::contracts::GalgamePipelineExecution;

pub(in crate::galgame::pipeline_run::pipeline) async fn plan_galgame_pipeline_assets(
    execution: &GalgamePipelineExecution<'_>,
    detail: &GalgameScriptDetail,
) -> Result<(), AppError> {
    let stage = GalgamePipelineStartStage::AssetGeneration;
    ensure_galgame_pipeline_not_cancelled(&execution.service.db, execution.options.workflow_run_id)
        .await?;
    mark_workflow_current_step(
        execution.service.db.connection(),
        execution.options.workflow_run_id,
        stage.key(),
    )
    .await
    .map_err(workflow_write_app_error)?;
    execution
        .service
        .report_pipeline_progress(
            execution.options.workflow_run_id,
            execution.options.task_id.as_deref(),
            execution.options.progress_reporter.as_ref(),
            stage,
            7,
            stage.message(),
        )
        .await?;

    let asset_plan = execution
        .service
        .plan_galgame_assets(execution.project_id)
        .await?;
    upsert_workflow_checkpoint(
        execution.service.db.connection(),
        UpsertWorkflowCheckpointInput {
            workflow_run_id: execution.options.workflow_run_id,
            step_key: CHECKPOINT_ASSET_GENERATION.to_string(),
            step_order: stage.order(),
            status: WORKFLOW_STATUS_COMPLETED.to_string(),
            input_fingerprint_sha256: None,
            input_fingerprint_json: Some(json!({
                "project_id": execution.project_id,
                "source_snapshot_id": execution.prompt_context.source_snapshot_id,
                "galgame_script_id": detail.script.id,
            })),
            checkpoint_json: Some(json!({
                "background_specs": asset_plan.summary.background_specs,
                "portrait_expression_specs": asset_plan.summary.portrait_expression_specs,
                "event_cg_specs": asset_plan.summary.event_cg_specs,
                "created_specs": asset_plan.summary.created_specs,
                "updated_specs": asset_plan.summary.updated_specs,
                "reused_specs": asset_plan.summary.reused_specs,
            })),
            artifacts_json: Some(json!({
                "galgame_script_id": detail.script.id,
                "generation_spec_count": asset_plan.summary.background_specs
                    + asset_plan.summary.portrait_expression_specs
                    + asset_plan.summary.event_cg_specs,
            })),
            generation_run_ids_json: None,
            error_message: None,
            completed: true,
        },
    )
    .await
    .map_err(workflow_write_app_error)?;
    execution
        .service
        .report_pipeline_progress(
            execution.options.workflow_run_id,
            execution.options.task_id.as_deref(),
            execution.options.progress_reporter.as_ref(),
            stage,
            7,
            "已创建 Galgame 资产生成意图",
        )
        .await?;
    Ok(())
}
