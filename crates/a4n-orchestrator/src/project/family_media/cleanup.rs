use a4n_core::error::AppError;
use chrono::Duration;

use crate::OrchestratorContext;

impl OrchestratorContext {
    /// 级联删除项目（含文本根派生守卫）。
    pub async fn delete_project_cascade(&self, project_id: i64) -> Result<(), AppError> {
        a4n_db::project_delete::delete_project_cascade_record(self.db.connection(), project_id)
            .await
            .map_err(crate::project_delete_error_to_app_error)
    }

    pub async fn claim_project_deletion_cleanup(
        &self,
        project_id: Option<i64>,
    ) -> Result<Option<a4n_db::cleanup_outbox::ClaimedProjectCleanup>, AppError> {
        a4n_db::cleanup_outbox::claim_project_deletion_cleanup(self.db.connection(), project_id)
            .await
            .map_err(AppError::Database)
    }

    pub async fn claim_media_workflow_v2_reset(
        &self,
        project_id: Option<i64>,
    ) -> Result<Option<a4n_db::cleanup_outbox::ClaimedMediaWorkflowV2Reset>, AppError> {
        a4n_db::cleanup_outbox::claim_media_workflow_v2_reset(self.db.connection(), project_id)
            .await
            .map_err(AppError::Database)
    }

    pub async fn reset_media_workflows_to_v2(
        &self,
        project_id: i64,
    ) -> Result<a4n_db::media_workflow_reset::MediaWorkflowV2ResetOutcome, AppError> {
        a4n_db::media_workflow_reset::reset_media_workflows_to_v2(self.db.connection(), project_id)
            .await
            .map_err(AppError::Database)
    }

    pub async fn ensure_media_workflow_v2_ready(&self, project_id: i64) -> Result<(), AppError> {
        if a4n_db::cleanup_outbox::media_workflow_v2_reset_pending(self.db.connection(), project_id)
            .await
            .map_err(AppError::Database)?
        {
            return Err(AppError::InvalidState {
                current: "media_workflow_contract_v2_reset_pending".to_string(),
                operation: "启动媒体生成流水线".to_string(),
            });
        }
        Ok(())
    }

    pub async fn complete_claimed_cleanup(
        &self,
        id: i64,
        lease_token: &str,
    ) -> Result<(), AppError> {
        a4n_db::cleanup_outbox::complete_claimed_cleanup(self.db.connection(), id, lease_token)
            .await
            .map_err(AppError::Database)
    }

    pub async fn renew_claimed_cleanup(&self, id: i64, lease_token: &str) -> Result<(), AppError> {
        a4n_db::cleanup_outbox::renew_claimed_cleanup(self.db.connection(), id, lease_token)
            .await
            .map_err(AppError::Database)
    }

    pub async fn retry_claimed_cleanup(
        &self,
        id: i64,
        lease_token: &str,
        error: String,
        retry_after_seconds: i64,
    ) -> Result<(), AppError> {
        a4n_db::cleanup_outbox::retry_claimed_cleanup(
            self.db.connection(),
            id,
            lease_token,
            error,
            Duration::seconds(retry_after_seconds),
        )
        .await
        .map_err(AppError::Database)
    }

    pub async fn unreferenced_media_paths(
        &self,
        paths: &[String],
    ) -> Result<Vec<String>, AppError> {
        a4n_db::media::filter_unreferenced_media_paths(self.db.connection(), paths)
            .await
            .map_err(AppError::Database)
    }

    pub async fn media_path_prefix_has_persisted_reference(
        &self,
        prefix: &str,
    ) -> Result<bool, AppError> {
        a4n_db::media::media_path_prefix_has_persisted_reference(self.db.connection(), prefix)
            .await
            .map_err(AppError::Database)
    }
}
