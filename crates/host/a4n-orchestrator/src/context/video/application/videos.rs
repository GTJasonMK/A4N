//! 视频记录 owner。

use super::*;

impl VideoApplication {
    pub(in crate::context) async fn create_generated_video_record(
        &self,
        input: CreateGeneratedVideoRecordDraftInput,
    ) -> Result<generated_video::Model, AppError> {
        a4n_db::video::create_generated_video_record(self.connection(), input)
            .await
            .map_err(video_db_error)
    }

    pub(in crate::context) async fn create_generated_video_record_with_generation_run(
        &self,
        input: CreateGeneratedVideoRecordDraftInput,
        generation_run: RecordStartedGenerationRunInput,
    ) -> Result<generated_video::Model, AppError> {
        a4n_db::video::create_generated_video_record_with_generation_run(
            self.connection(),
            input,
            generation_run,
        )
        .await
        .map_err(video_db_error)
    }

    pub(in crate::context) async fn create_prompt_generated_video_record_with_generation_run(
        &self,
        input: CreateGeneratedVideoRecordDraftInput,
        generation_run: RecordStartedGenerationRunInput,
        prompt_id: i64,
    ) -> Result<generated_video::Model, AppError> {
        a4n_db::video::create_prompt_generated_video_record_with_generation_run(
            self.connection(),
            input,
            generation_run,
            prompt_id,
        )
        .await
        .map_err(video_db_error)
    }

    pub(in crate::context) async fn generated_videos_for_chapter(
        &self,
        project_id: i64,
        chapter_id: i64,
    ) -> Result<Vec<generated_video::Model>, AppError> {
        a4n_db::video::generated_videos_for_chapter(self.connection(), project_id, chapter_id)
            .await
            .map_err(AppError::database)
    }

    pub(in crate::context) async fn generated_video_by_id(
        &self,
        video_id: i64,
    ) -> Result<Option<generated_video::Model>, AppError> {
        a4n_db::video::generated_video_by_id(self.connection(), video_id)
            .await
            .map_err(AppError::database)
    }

    pub(in crate::context) async fn generated_videos_by_ids(
        &self,
        video_ids: &[i64],
    ) -> Result<Vec<generated_video::Model>, AppError> {
        a4n_db::video::generated_videos_by_ids(self.connection(), video_ids)
            .await
            .map_err(AppError::database)
    }

    pub(in crate::context) async fn generated_videos_by_provider_job_id(
        &self,
        project_id: Option<i64>,
        provider_job_id: &str,
    ) -> Result<Vec<generated_video::Model>, AppError> {
        a4n_db::video::generated_videos_by_provider_job_id(
            self.connection(),
            project_id,
            provider_job_id,
        )
        .await
        .map_err(AppError::database)
    }

    pub(in crate::context) async fn update_generated_video_record(
        &self,
        video_id: i64,
        input: UpdateGeneratedVideoRecordInput,
    ) -> Result<generated_video::Model, AppError> {
        a4n_db::video::update_generated_video_record(self.connection(), video_id, input)
            .await
            .map_err(video_db_error)
    }

    pub(in crate::context) async fn delete_generated_video_record_with_cleanup(
        &self,
        video_id: i64,
    ) -> Result<(), AppError> {
        a4n_db::video::delete_generated_video_record_with_cleanup(self.connection(), video_id)
            .await
            .map_err(video_db_error)
    }

    pub(in crate::context) async fn delete_missing_generated_video_references_record(
        &self,
        video_id: i64,
    ) -> Result<u64, AppError> {
        a4n_db::video::delete_missing_generated_video_references_record(self.connection(), video_id)
            .await
            .map_err(video_db_error)
    }

    pub(in crate::context) async fn missing_video_reference_project_ids(
        &self,
        video_id: i64,
    ) -> Result<Vec<i64>, AppError> {
        a4n_db::video::missing_video_reference_project_ids(self.connection(), video_id)
            .await
            .map_err(video_db_error)
    }

    pub(in crate::context) async fn update_generated_video_waiting_status_and_prompt_record(
        &self,
        video_id: i64,
        prompt_id: Option<i64>,
        status: &str,
    ) -> Result<generated_video::Model, AppError> {
        a4n_db::video::update_generated_video_waiting_status_and_prompt_record(
            self.connection(),
            video_id,
            prompt_id,
            status,
        )
        .await
        .map_err(video_db_error)
    }

    pub(in crate::context) async fn mark_generated_video_error_status_and_prompt_record(
        &self,
        video_id: i64,
        prompt_id: Option<i64>,
        status: &str,
        message: &str,
    ) -> Result<generated_video::Model, AppError> {
        a4n_db::video::mark_generated_video_error_status_and_prompt_record(
            self.connection(),
            video_id,
            prompt_id,
            status,
            message,
        )
        .await
        .map_err(video_db_error)
    }

    pub(in crate::context) async fn mark_generated_videos_error_status_and_prompt_records(
        &self,
        inputs: &[GeneratedVideoErrorStatusBatchInput],
        message: &str,
    ) -> Result<Vec<i64>, AppError> {
        a4n_db::video::mark_generated_videos_error_status_and_prompt_records(
            self.connection(),
            inputs,
            message,
        )
        .await
        .map_err(video_db_error)
    }

    pub(in crate::context) async fn mark_generated_video_job_and_prompt_record(
        &self,
        video_id: i64,
        provider_job_id: &str,
        metadata_json: Option<Value>,
        prompt_id: Option<i64>,
    ) -> Result<generated_video::Model, AppError> {
        a4n_db::video::mark_generated_video_job_and_prompt_record(
            self.connection(),
            video_id,
            provider_job_id,
            metadata_json,
            prompt_id,
        )
        .await
        .map_err(video_db_error)
    }

    pub(in crate::context) async fn mark_generated_video_completed_and_link_prompt_record(
        &self,
        video_id: i64,
        file_path: &str,
        thumbnail_path: Option<&str>,
        properties: &a4n_domain::media::VideoFileProperties,
        metadata_json: Option<Value>,
        prompt_id: Option<i64>,
    ) -> Result<generated_video::Model, AppError> {
        a4n_db::video::mark_generated_video_completed_and_link_prompt_record(
            self.connection(),
            video_id,
            file_path,
            thumbnail_path,
            properties,
            metadata_json,
            prompt_id,
        )
        .await
        .map_err(video_db_error)
    }
}
