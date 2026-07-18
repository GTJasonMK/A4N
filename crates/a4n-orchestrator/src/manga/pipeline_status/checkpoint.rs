use a4n_core::error::AppError;
use a4n_db::manga::{self as manga_db, SaveMangaPipelineCheckpointInput};

use super::super::OrchestratorContext;
use super::contracts::MangaPipelineCheckpointInput;
use super::status::{manga_pipeline_db_error, to_set_input};

impl OrchestratorContext {
    pub(in crate::manga) async fn clear_pipeline_checkpoint(
        &self,
        project_id: i64,
        chapter_id: i64,
    ) -> Result<(), AppError> {
        manga_db::clear_manga_pipeline_checkpoint(self.db.connection(), project_id, chapter_id)
            .await
            .map_err(manga_pipeline_db_error)
    }

    pub(in crate::manga) async fn save_pipeline_checkpoint(
        &self,
        input: MangaPipelineCheckpointInput<'_>,
    ) -> Result<(), AppError> {
        manga_db::save_manga_pipeline_checkpoint(
            self.db.connection(),
            SaveMangaPipelineCheckpointInput {
                status: to_set_input(input.status),
                source_metadata: input.source_metadata.clone(),
                reference_assets_metadata: input.reference_assets_metadata.clone(),
                prompt_fingerprint: input.prompt_fingerprint.clone(),
                checkpoint_data: input.checkpoint_data.clone(),
            },
        )
        .await
        .map_err(manga_pipeline_db_error)
    }
}
