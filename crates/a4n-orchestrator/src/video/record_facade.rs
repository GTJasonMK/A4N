use super::*;

impl OrchestratorContext {
    pub async fn create_generated_image_record(
        &self,
        input: CreateGeneratedImageRecordDraftInput,
    ) -> Result<generated_image::Model, AppError> {
        a4n_db::video::create_generated_image_record(self.db.connection(), input)
            .await
            .map_err(video_db_error)
    }

    /// 创建图片并写入 completed generation run checkpoint。
    pub async fn create_generated_image_record_with_generation_run_checkpoint(
        &self,
        input: CreateGeneratedImageRecordDraftInput,
        generation_run: a4n_db::generation::RecordCompletedGenerationRunWithCheckpointInput,
    ) -> Result<generated_image::Model, AppError> {
        a4n_db::video::create_generated_image_record_with_generation_run_checkpoint(
            self.db.connection(),
            input,
            generation_run,
        )
        .await
        .map_err(video_db_error)
    }

    pub async fn generated_video_ids_for_chapter(
        &self,
        project_id: i64,
        chapter_id: i64,
    ) -> Result<Vec<i64>, AppError> {
        a4n_db::video::generated_video_ids_for_chapter(self.db.connection(), project_id, chapter_id)
            .await
            .map_err(video_db_error)
    }

    pub async fn generated_videos_for_chapter(
        &self,
        project_id: i64,
        chapter_id: i64,
    ) -> Result<Vec<generated_video::Model>, AppError> {
        a4n_db::video::generated_videos_for_chapter(self.db.connection(), project_id, chapter_id)
            .await
            .map_err(AppError::Database)
    }

    pub async fn generated_video_by_id(
        &self,
        video_id: i64,
    ) -> Result<Option<generated_video::Model>, AppError> {
        a4n_db::video::generated_video_by_id(self.db.connection(), video_id)
            .await
            .map_err(AppError::Database)
    }

    pub async fn toggle_generated_video_favorite(
        &self,
        video_id: i64,
    ) -> Result<generated_video::Model, AppError> {
        a4n_db::video::toggle_generated_video_favorite_record(self.db.connection(), video_id)
            .await
            .map_err(video_db_error)
    }

    pub async fn update_generated_video_record(
        &self,
        video_id: i64,
        input: UpdateGeneratedVideoRecordInput,
    ) -> Result<generated_video::Model, AppError> {
        a4n_db::video::update_generated_video_record(self.db.connection(), video_id, input)
            .await
            .map_err(video_db_error)
    }

    pub async fn delete_generated_video_record(&self, video_id: i64) -> Result<(), AppError> {
        a4n_db::video::delete_generated_video_record(self.db.connection(), video_id)
            .await
            .map_err(video_db_error)
    }

    pub async fn delete_missing_generated_video_references(
        &self,
        video_id: i64,
    ) -> Result<u64, AppError> {
        a4n_db::video::delete_missing_generated_video_references_record(
            self.db.connection(),
            video_id,
        )
        .await
        .map_err(video_db_error)
    }

    pub async fn missing_video_reference_project_ids(
        &self,
        video_id: i64,
    ) -> Result<Vec<i64>, AppError> {
        if let Some(video) = a4n_db::video::generated_video_by_id(self.db.connection(), video_id)
            .await
            .map_err(AppError::Database)?
        {
            return Ok(vec![video.project_id]);
        }
        a4n_db::video::missing_video_reference_project_ids(self.db.connection(), video_id)
            .await
            .map_err(video_db_error)
    }
}
