use a4n_core::error::AppError;
use a4n_core::event::progress::{ProgressMeta, ProgressPhaseKind};
use a4n_db::{
    DbHandle,
    workflow::{
        WORKFLOW_STATUS_CANCELLED, WorkflowProgressEventInput, record_workflow_progress_event,
        workflow_run_model_by_id,
    },
};

use crate::workflow::{workflow_read_app_error, workflow_write_app_error};

use super::super::{GALGAME_PIPELINE_TOTAL_STEPS, GalgamePipelineStartStage, OrchestratorContext};
use super::contracts::{GalgamePipelineProgressUpdate, GalgameProgressReporter};

impl OrchestratorContext {
    pub(in crate::galgame) async fn report_pipeline_progress(
        &self,
        workflow_run_id: i64,
        task_id: Option<&str>,
        progress_reporter: Option<&GalgameProgressReporter>,
        stage: GalgamePipelineStartStage,
        current: u32,
        message: &str,
    ) -> Result<(), AppError> {
        ensure_galgame_pipeline_not_cancelled(&self.database, workflow_run_id).await?;
        let update = galgame_progress_update(stage, current, message)?;
        record_workflow_progress_event(
            self.database.connection(),
            WorkflowProgressEventInput {
                workflow_run_id,
                task_id,
                current: update.current,
                total: update.total,
                message,
                meta: Some(update.meta.clone()),
            },
        )
        .await
        .map_err(workflow_write_app_error)?;
        if let Some(reporter) = progress_reporter {
            reporter(update)?;
        }
        Ok(())
    }
}

pub(crate) fn galgame_progress_update(
    stage: GalgamePipelineStartStage,
    current: u32,
    message: &str,
) -> Result<GalgamePipelineProgressUpdate, AppError> {
    if current == 0 || current > GALGAME_PIPELINE_TOTAL_STEPS {
        return Err(AppError::InvalidArgument(format!(
            "Galgame 管线进度 current 必须在 1..={} 范围内，实际为 {current}",
            GALGAME_PIPELINE_TOTAL_STEPS
        )));
    }
    Ok(GalgamePipelineProgressUpdate {
        current,
        total: GALGAME_PIPELINE_TOTAL_STEPS,
        message: message.to_string(),
        meta: galgame_progress_meta(stage, message),
    })
}

fn galgame_progress_meta(stage: GalgamePipelineStartStage, message: &str) -> ProgressMeta {
    let phase = match stage {
        GalgamePipelineStartStage::ShootingScriptProjection
        | GalgamePipelineStartStage::ScriptAdaptation
        | GalgamePipelineStartStage::CharacterStaging
        | GalgamePipelineStartStage::VisualPlanning
        | GalgamePipelineStartStage::BranchDesign => ProgressPhaseKind::ModelWait,
        GalgamePipelineStartStage::PersistScript => ProgressPhaseKind::Save,
        GalgamePipelineStartStage::AssetGeneration => ProgressPhaseKind::AssetGeneration,
    };
    ProgressMeta::new(
        "galgame_generation",
        stage.key(),
        galgame_stage_label(stage),
        phase,
    )
    .with_detail(message)
}

fn galgame_stage_label(stage: GalgamePipelineStartStage) -> &'static str {
    match stage {
        GalgamePipelineStartStage::ShootingScriptProjection => "拍摄剧本投影",
        GalgamePipelineStartStage::ScriptAdaptation => "剧本改编",
        GalgamePipelineStartStage::CharacterStaging => "角色演出",
        GalgamePipelineStartStage::VisualPlanning => "视觉规划",
        GalgamePipelineStartStage::BranchDesign => "分支设计",
        GalgamePipelineStartStage::PersistScript => "写入剧本",
        GalgamePipelineStartStage::AssetGeneration => "资产意图",
    }
}

pub(super) async fn ensure_galgame_pipeline_not_cancelled(
    db: &DbHandle,
    workflow_run_id: i64,
) -> Result<(), AppError> {
    let run = workflow_run_model_by_id(db.connection(), workflow_run_id)
        .await
        .map_err(workflow_read_app_error)?;
    if run.status == WORKFLOW_STATUS_CANCELLED {
        return Err(AppError::Cancelled);
    }
    Ok(())
}
