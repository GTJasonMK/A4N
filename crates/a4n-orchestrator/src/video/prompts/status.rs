use a4n_core::{error::AppError, models::ai::generated_video};
use serde_json::Value;

use super::super::{OrchestratorContext, video_db_error};

impl OrchestratorContext {
    pub async fn update_generated_video_waiting_status_and_prompt(
        &self,
        video_id: i64,
        prompt_id: Option<i64>,
        status: &str,
    ) -> Result<generated_video::Model, AppError> {
        a4n_db::video::update_generated_video_waiting_status_and_prompt_record(
            self.db.connection(),
            video_id,
            prompt_id,
            status,
        )
        .await
        .map_err(video_db_error)
    }

    pub async fn mark_generated_video_error_status_and_prompt(
        &self,
        video_id: i64,
        prompt_id: Option<i64>,
        status: &str,
        message: &str,
    ) -> Result<generated_video::Model, AppError> {
        let updated = a4n_db::video::mark_generated_video_error_status_and_prompt_record(
            self.db.connection(),
            video_id,
            prompt_id,
            status,
            message,
        )
        .await
        .map_err(video_db_error)?;
        // failed 路径与 services 侧一致：回写后同步项目资产 lineage。
        if status == "failed" {
            self.sync_project_asset_lineage(updated.project_id).await?;
        }
        Ok(updated)
    }

    pub async fn mark_generated_video_job_and_prompt(
        &self,
        video_id: i64,
        provider_job_id: &str,
        metadata_json: Option<Value>,
        prompt_id: Option<i64>,
    ) -> Result<generated_video::Model, AppError> {
        let updated = a4n_db::video::mark_generated_video_job_and_prompt_record(
            self.db.connection(),
            video_id,
            provider_job_id,
            metadata_json,
            prompt_id,
        )
        .await
        .map_err(video_db_error)?;
        Ok(updated)
    }

    pub async fn mark_generated_video_completed_and_link_prompt(
        &self,
        video_id: i64,
        file_path: &str,
        thumbnail_path: Option<&str>,
        file_size: i64,
        metadata_json: Option<Value>,
        prompt_id: Option<i64>,
    ) -> Result<generated_video::Model, AppError> {
        let updated = a4n_db::video::mark_generated_video_completed_and_link_prompt_record(
            self.db.connection(),
            video_id,
            file_path,
            thumbnail_path,
            file_size,
            metadata_json,
            prompt_id,
        )
        .await
        .map_err(video_db_error)?;
        // completed 路径与 services 侧一致：回写后同步项目资产 lineage。
        self.sync_project_asset_lineage(updated.project_id).await?;
        Ok(updated)
    }
}
