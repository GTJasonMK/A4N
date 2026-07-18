use a4n_core::{error::AppError, models::ai::generated_video};
use a4n_db::generation::RecordStartedGenerationRunInput;
use a4n_db::video::CreateGeneratedVideoRecordDraftInput;

use super::super::OrchestratorContext;
use crate::video::video_db_error;

impl OrchestratorContext {
    pub async fn create_video_record(
        &self,
        input: CreateGeneratedVideoRecordDraftInput,
    ) -> Result<generated_video::Model, AppError> {
        a4n_db::video::create_generated_video_record(self.db.connection(), input)
            .await
            .map_err(video_db_error)
    }

    pub async fn create_video_record_with_generation_run(
        &self,
        input: CreateGeneratedVideoRecordDraftInput,
        generation_run: RecordStartedGenerationRunInput,
    ) -> Result<generated_video::Model, AppError> {
        a4n_db::video::create_generated_video_record_with_generation_run(
            self.db.connection(),
            input,
            generation_run,
        )
        .await
        .map_err(video_db_error)
    }

    pub async fn create_prompt_video_record_with_generation_run(
        &self,
        input: CreateGeneratedVideoRecordDraftInput,
        generation_run: RecordStartedGenerationRunInput,
        prompt_id: i64,
    ) -> Result<generated_video::Model, AppError> {
        a4n_db::video::create_prompt_generated_video_record_with_generation_run(
            self.db.connection(),
            input,
            generation_run,
            prompt_id,
        )
        .await
        .map_err(video_db_error)
    }
}
