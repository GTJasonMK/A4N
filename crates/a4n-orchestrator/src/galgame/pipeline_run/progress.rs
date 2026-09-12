use a4n_api_contract::galgame::{GalgamePipelineProgressUpdate, GalgameProgressReporter};
use a4n_db::workflow::WorkflowProgressEventInput;
use a4n_domain::progress::{ProgressMeta, ProgressPhaseKind};
use a4n_domain::workflow::WORKFLOW_STATUS_CANCELLED;
use a4n_error::AppError;

use super::super::{GALGAME_PIPELINE_TOTAL_STEPS, GalgamePipelineStartStage, OrchestratorContext};

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
        ensure_galgame_pipeline_not_cancelled(self, workflow_run_id).await?;
        let update = galgame_progress_update(stage, current, message)?;
        self.record_workflow_progress_event(WorkflowProgressEventInput {
            workflow_run_id,
            task_id,
            current: update.current,
            total: update.total,
            message,
            meta: Some(update.meta.clone()),
        })
        .await?;
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
    service: &OrchestratorContext,
    workflow_run_id: i64,
) -> Result<(), AppError> {
    let run = service
        .workflow_run_status_record_by_id(workflow_run_id)
        .await?;
    if run.status == WORKFLOW_STATUS_CANCELLED {
        return Err(AppError::Cancelled);
    }
    Ok(())
}
