use a4n_core::{error::AppError, models::chapter::chapter_video_prompt};

use super::super::source::validate_import_chapter_video_prompt;
use super::super::{OrchestratorContext, video_db_error};

impl OrchestratorContext {
    pub async fn chapter_video_prompt(
        &self,
        prompt_id: i64,
    ) -> Result<chapter_video_prompt::Model, AppError> {
        a4n_db::video::chapter_video_prompt_by_id(self.db.connection(), prompt_id)
            .await
            .map_err(video_db_error)?
            .ok_or_else(|| AppError::NotFound("视频提示词不存在".to_string()))
    }

    pub async fn chapter_video_prompt_for_import(
        &self,
        project_id: i64,
        chapter_id: i64,
        prompt_id: i64,
    ) -> Result<chapter_video_prompt::Model, AppError> {
        let prompt = a4n_db::video::chapter_video_prompt_by_id(self.db.connection(), prompt_id)
            .await
            .map_err(video_db_error)?
            .ok_or_else(|| AppError::NotFound(format!("视频提示词不存在: {prompt_id}")))?;
        validate_import_chapter_video_prompt(&prompt, project_id, chapter_id)
            .map_err(AppError::InvalidArgument)?;
        Ok(prompt)
    }

    pub async fn chapter_video_prompts_for_chapter_by_id(
        &self,
        project_id: i64,
        chapter_id: i64,
    ) -> Result<Vec<chapter_video_prompt::Model>, AppError> {
        a4n_db::video::chapter_video_prompts_for_chapter_by_id(
            self.db.connection(),
            project_id,
            chapter_id,
        )
        .await
        .map_err(video_db_error)
    }

    pub async fn chapter_video_prompts_for_chapter(
        &self,
        project_id: i64,
        chapter_id: i64,
    ) -> Result<Vec<chapter_video_prompt::Model>, AppError> {
        a4n_db::video::chapter_video_prompts_for_chapter(
            self.db.connection(),
            project_id,
            chapter_id,
        )
        .await
        .map_err(AppError::Database)
    }

    pub async fn chapter_video_prompts_for_batch_generation(
        &self,
        prompt_ids: &[i64],
    ) -> Result<Vec<chapter_video_prompt::Model>, AppError> {
        a4n_db::video::chapter_video_prompts_for_batch_generation(self.db.connection(), prompt_ids)
            .await
            .map_err(video_db_error)
    }

    pub async fn chapter_video_boundary_frame_prompt_candidates(
        &self,
        project_id: i64,
        chapter_id: i64,
        first_frame_number: i32,
        last_frame_number: i32,
    ) -> Result<Vec<chapter_video_prompt::Model>, AppError> {
        a4n_db::video::chapter_video_boundary_frame_prompt_candidates(
            self.db.connection(),
            project_id,
            chapter_id,
            first_frame_number,
            last_frame_number,
        )
        .await
        .map_err(video_db_error)
    }

    pub async fn chapter_video_first_frame_prompt_candidates(
        &self,
        project_id: i64,
        chapter_id: i64,
    ) -> Result<Vec<chapter_video_prompt::Model>, AppError> {
        a4n_db::video::chapter_video_first_frame_prompt_candidates(
            self.db.connection(),
            project_id,
            chapter_id,
        )
        .await
        .map_err(video_db_error)
    }

    pub async fn chapter_video_first_frame_chain_prompt_candidates(
        &self,
        project_id: i64,
        chapter_id: i64,
    ) -> Result<Vec<chapter_video_prompt::Model>, AppError> {
        a4n_db::video::chapter_video_first_frame_chain_prompt_candidates(
            self.db.connection(),
            project_id,
            chapter_id,
        )
        .await
        .map_err(video_db_error)
    }

    pub async fn chapter_video_storyboard_prompt_candidates(
        &self,
        project_id: i64,
        chapter_id: i64,
        shot_number: i32,
    ) -> Result<Vec<chapter_video_prompt::Model>, AppError> {
        a4n_db::video::chapter_video_storyboard_prompt_candidates(
            self.db.connection(),
            project_id,
            chapter_id,
            shot_number,
        )
        .await
        .map_err(video_db_error)
    }

    pub async fn has_running_chapter_video_prompts(
        &self,
        project_id: i64,
        chapter_id: i64,
    ) -> Result<bool, AppError> {
        a4n_db::video::has_running_chapter_video_prompts(
            self.db.connection(),
            project_id,
            chapter_id,
        )
        .await
        .map_err(video_db_error)
    }
}
