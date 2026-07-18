use a4n_core::{error::AppError, models::chapter::chapter_video_prompt};
use a4n_db::generation::RecordCompletedGenerationRunWithCheckpointInput;
use serde_json::Value;

use super::super::{OrchestratorContext, UpdateChapterVideoPromptInput, video_db_error};

impl OrchestratorContext {
    pub async fn update_chapter_video_prompt_with_policy(
        &self,
        prompt_id: i64,
        input: UpdateChapterVideoPromptInput,
        allow_downstream: bool,
    ) -> Result<chapter_video_prompt::Model, AppError> {
        a4n_db::video_prompt::update_chapter_video_prompt_with_policy(
            self.db.connection(),
            prompt_id,
            input,
            allow_downstream,
        )
        .await
        .map_err(video_db_error)
    }

    pub async fn update_chapter_video_prompt_with_generation_run_checkpoint(
        &self,
        prompt_id: i64,
        input: UpdateChapterVideoPromptInput,
        metadata_json: Option<Value>,
        allow_downstream: bool,
        generation_run: RecordCompletedGenerationRunWithCheckpointInput,
    ) -> Result<chapter_video_prompt::Model, AppError> {
        a4n_db::video_prompt::update_chapter_video_prompt_with_generation_run_checkpoint(
            self.db.connection(),
            prompt_id,
            input,
            metadata_json,
            allow_downstream,
            generation_run,
        )
        .await
        .map_err(video_db_error)
    }

    /// 删除单条视频提示词；`allow_downstream` 时跳过下游可变性检查。
    pub async fn delete_chapter_video_prompt_checked(
        &self,
        prompt_id: i64,
        allow_downstream: bool,
    ) -> Result<(), AppError> {
        a4n_db::video_prompt::delete_chapter_video_prompt_with_policy(
            self.db.connection(),
            prompt_id,
            allow_downstream,
        )
        .await
        .map_err(video_db_error)
    }

    /// 批量删除视频提示词；`allow_downstream` 时跳过下游可变性检查。
    pub async fn delete_chapter_video_prompts_checked(
        &self,
        prompt_ids: &[i64],
        allow_downstream: bool,
    ) -> Result<u64, AppError> {
        a4n_db::video_prompt::delete_chapter_video_prompts_with_policy(
            self.db.connection(),
            prompt_ids,
            allow_downstream,
        )
        .await
        .map_err(video_db_error)
    }

    pub async fn prompt_downstream_dependencies(
        &self,
        prompt_id: i64,
    ) -> Result<a4n_db::video_prompt::VideoPromptDownstreamDependencies, AppError> {
        a4n_db::video_prompt::prompt_downstream_dependencies(self.db.connection(), prompt_id)
            .await
            .map_err(video_db_error)
    }

    pub async fn ensure_frame_prompt_image_generation_allowed(
        &self,
        prompt_id: i64,
        generation_source: &str,
    ) -> Result<(), AppError> {
        a4n_db::video_prompt::ensure_frame_prompt_image_generation_allowed(
            self.db.connection(),
            prompt_id,
            generation_source,
        )
        .await
        .map_err(video_db_error)
    }

    pub async fn effective_frame_prompt_image_reference_ids(
        &self,
        prompt_id: i64,
        generation_source: &str,
        requested_reference_image_ids: &[i64],
    ) -> Result<Vec<i64>, AppError> {
        a4n_db::video_prompt::effective_frame_prompt_image_reference_ids(
            self.db.connection(),
            prompt_id,
            generation_source,
            requested_reference_image_ids,
        )
        .await
        .map_err(video_db_error)
    }
}
