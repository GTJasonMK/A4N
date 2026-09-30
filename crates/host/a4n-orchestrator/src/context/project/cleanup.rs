use super::super::OrchestratorContext;
use a4n_error::AppError;

#[async_trait::async_trait]
impl a4n_cleanup_contract::CleanupOutboxApplicationPort for OrchestratorContext {
    async fn insert_file_artifact_cleanup_intent(
        &self,
        project_id: i64,
        dedupe_key: String,
        paths: Vec<String>,
    ) -> Result<(), AppError> {
        OrchestratorContext::insert_file_artifact_cleanup_intent(
            self, project_id, dedupe_key, paths,
        )
        .await
    }

    async fn claim_project_deletion_cleanup(
        &self,
        project_id: Option<i64>,
    ) -> Result<Option<a4n_cleanup_contract::ProjectDeletionCleanupClaim>, AppError> {
        OrchestratorContext::claim_project_deletion_cleanup(self, project_id).await
    }

    async fn claim_file_artifact_cleanup(
        &self,
        project_id: Option<i64>,
    ) -> Result<Option<a4n_cleanup_contract::FileArtifactCleanupClaim>, AppError> {
        OrchestratorContext::claim_file_artifact_cleanup(self, project_id).await
    }

    async fn claim_generation_cleanup(
        &self,
        project_id: Option<i64>,
    ) -> Result<Option<a4n_cleanup_contract::GenerationCleanupClaim>, AppError> {
        OrchestratorContext::claim_generation_cleanup(self, project_id).await
    }

    async fn project_ids_with_incomplete_cleanup(&self) -> Result<Vec<i64>, AppError> {
        OrchestratorContext::project_ids_with_incomplete_cleanup(self).await
    }

    async fn purge_completed_cleanup_audits(&self, limit: usize) -> Result<u64, AppError> {
        OrchestratorContext::purge_completed_cleanup_audits(self, limit).await
    }

    async fn complete_claimed_cleanup(&self, id: i64, lease_token: &str) -> Result<(), AppError> {
        OrchestratorContext::complete_claimed_cleanup(self, id, lease_token).await
    }

    async fn complete_claimed_generation_cleanup(
        &self,
        id: i64,
        lease_token: &str,
        outcome: a4n_cleanup_contract::GenerationCleanupOutcome,
    ) -> Result<(), AppError> {
        OrchestratorContext::complete_claimed_generation_cleanup(self, id, lease_token, outcome)
            .await
    }

    async fn observe_claimed_generation_reconcile(
        &self,
        id: i64,
        lease_token: &str,
        retry_after_seconds: i64,
    ) -> Result<a4n_cleanup_contract::GenerationCleanupObservationOutcome, AppError> {
        OrchestratorContext::observe_claimed_generation_reconcile(
            self,
            id,
            lease_token,
            retry_after_seconds,
        )
        .await
    }

    async fn schedule_provider_delete_from_reconcile(
        &self,
        id: i64,
        lease_token: &str,
    ) -> Result<(), AppError> {
        OrchestratorContext::schedule_provider_delete_from_reconcile(self, id, lease_token).await
    }

    async fn renew_claimed_cleanup(&self, id: i64, lease_token: &str) -> Result<(), AppError> {
        OrchestratorContext::renew_claimed_cleanup(self, id, lease_token).await
    }

    async fn retry_claimed_cleanup(
        &self,
        id: i64,
        lease_token: &str,
        error: String,
        retry_after_seconds: i64,
    ) -> Result<(), AppError> {
        OrchestratorContext::retry_claimed_cleanup(
            self,
            id,
            lease_token,
            error,
            retry_after_seconds,
        )
        .await
    }

    async fn retry_or_fail_claimed_generation_cleanup(
        &self,
        id: i64,
        lease_token: &str,
        error: String,
    ) -> Result<a4n_cleanup_contract::GenerationCleanupRetryOutcome, AppError> {
        OrchestratorContext::retry_or_fail_claimed_generation_cleanup(self, id, lease_token, error)
            .await
    }
}
