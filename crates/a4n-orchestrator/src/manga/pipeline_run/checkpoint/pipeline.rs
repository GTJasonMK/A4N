use a4n_core::error::AppError;
use serde_json::Value;

use super::super::super::pipeline_status::{
    MangaPipelineCheckpointInput, MangaPipelineStatusInput,
};
use super::super::super::{OrchestratorContext, PIPELINE_TOTAL_STEPS};
use super::super::contracts::MangaRunningPipelineCheckpointInput;

impl OrchestratorContext {
    pub(in crate::manga::pipeline_run) async fn save_running_pipeline_checkpoint(
        &self,
        input: MangaRunningPipelineCheckpointInput<'_>,
    ) -> Result<(), AppError> {
        let checkpoint_data = Value::Object(input.checkpoint.clone());
        self.save_pipeline_checkpoint(MangaPipelineCheckpointInput {
            status: MangaPipelineStatusInput {
                chapter_id: input.chapter_id,
                project_id: input.project_id,
                pipeline_status: "running",
                stage: input.stage,
                current: input.current,
                total: PIPELINE_TOTAL_STEPS,
                message: input.message,
                task_id: input.task_id,
                allow_cancel_override: false,
            },
            source_metadata: input.source_metadata,
            reference_assets_metadata: input.reference_assets_metadata,
            prompt_fingerprint: input.prompt_fingerprint,
            checkpoint_data: &checkpoint_data,
        })
        .await
    }
}
