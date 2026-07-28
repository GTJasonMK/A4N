use a4n_core::{error::AppError, models::project::novel_project};
use a4n_orchestrator::{
    OrchestratorContext,
    project::{
        ExportDerivedProjectPackageApplicationInput, ExportProjectFamilyPackageApplicationInput,
        ExportedProjectFamilyPackage, ImportProjectFamilyPackageApplicationInput,
        ProjectFamilyContext, ProjectFamilyTransferError, ProjectFamilyTransferHostPort,
        SourceSnapshotChapterSummary,
    },
};

pub(crate) struct ProjectFamilyRuntimeFacade<'a> {
    orchestrator: &'a OrchestratorContext,
}

impl<'a> ProjectFamilyRuntimeFacade<'a> {
    pub(super) fn new(orchestrator: &'a OrchestratorContext) -> Self {
        Self { orchestrator }
    }

    pub(crate) async fn character_portrait_project_id(
        &self,
        portrait_id: i64,
    ) -> Result<i64, AppError> {
        self.orchestrator
            .character_portrait_project_id(portrait_id)
            .await
    }

    pub(crate) async fn protagonist_profile_project_id(
        &self,
        profile_id: i64,
    ) -> Result<i64, AppError> {
        self.orchestrator
            .protagonist_profile_project_id(profile_id)
            .await
    }

    pub(crate) async fn derived_project_family_context(
        &self,
        project_id: i64,
    ) -> Result<ProjectFamilyContext, AppError> {
        self.orchestrator
            .derived_project_family_context(project_id)
            .await
    }

    pub(crate) async fn source_snapshot_chapters(
        &self,
        project_id: i64,
    ) -> Result<Vec<SourceSnapshotChapterSummary>, AppError> {
        self.orchestrator.source_snapshot_chapters(project_id).await
    }

    pub(crate) async fn export_project_family_package_validated<Host>(
        &self,
        input: ExportProjectFamilyPackageApplicationInput,
        host_port: &Host,
    ) -> Result<ExportedProjectFamilyPackage, ProjectFamilyTransferError>
    where
        Host: ProjectFamilyTransferHostPort,
    {
        self.orchestrator
            .export_project_family_package_validated(input, host_port)
            .await
    }

    pub(crate) async fn export_derived_project_package_validated<Host>(
        &self,
        input: ExportDerivedProjectPackageApplicationInput,
        host_port: &Host,
    ) -> Result<ExportedProjectFamilyPackage, ProjectFamilyTransferError>
    where
        Host: ProjectFamilyTransferHostPort,
    {
        self.orchestrator
            .export_derived_project_package_validated(input, host_port)
            .await
    }

    pub(crate) async fn import_project_family_package_validated<Host>(
        &self,
        input: ImportProjectFamilyPackageApplicationInput,
        host_port: &mut Host,
    ) -> Result<Vec<novel_project::Model>, ProjectFamilyTransferError>
    where
        Host: ProjectFamilyTransferHostPort,
    {
        self.orchestrator
            .import_project_family_package_validated(input, host_port)
            .await
    }

    pub(crate) async fn delete_project_cascade_validated(
        &self,
        project_id: i64,
    ) -> Result<(), AppError> {
        self.orchestrator
            .delete_project_cascade_validated(project_id)
            .await
    }

    pub(crate) async fn claim_project_deletion_cleanup(
        &self,
        project_id: Option<i64>,
    ) -> Result<Option<a4n_db::cleanup_outbox::ClaimedProjectCleanup>, AppError> {
        self.orchestrator
            .claim_project_deletion_cleanup(project_id)
            .await
    }

    pub(crate) async fn claim_media_workflow_v2_reset(
        &self,
        project_id: Option<i64>,
    ) -> Result<Option<a4n_db::cleanup_outbox::ClaimedMediaWorkflowV2Reset>, AppError> {
        self.orchestrator
            .claim_media_workflow_v2_reset(project_id)
            .await
    }

    pub(crate) async fn insert_file_artifact_cleanup_intent(
        &self,
        project_id: i64,
        dedupe_key: String,
        paths: Vec<String>,
    ) -> Result<(), AppError> {
        self.orchestrator
            .insert_file_artifact_cleanup_intent(project_id, dedupe_key, paths)
            .await
    }

    pub(crate) async fn claim_file_artifact_cleanup(
        &self,
        project_id: Option<i64>,
    ) -> Result<Option<a4n_db::cleanup_outbox::ClaimedFileArtifactCleanup>, AppError> {
        self.orchestrator
            .claim_file_artifact_cleanup(project_id)
            .await
    }

    pub(crate) async fn claim_generation_cleanup(
        &self,
        project_id: Option<i64>,
    ) -> Result<Option<a4n_db::cleanup_outbox::ClaimedGenerationCleanup>, AppError> {
        self.orchestrator.claim_generation_cleanup(project_id).await
    }

    pub(crate) async fn project_ids_with_incomplete_cleanup(&self) -> Result<Vec<i64>, AppError> {
        self.orchestrator
            .project_ids_with_incomplete_cleanup()
            .await
    }

    pub(crate) async fn purge_completed_cleanup_audits(
        &self,
        limit: usize,
    ) -> Result<u64, AppError> {
        self.orchestrator
            .purge_completed_cleanup_audits(limit)
            .await
    }

    pub(crate) async fn reset_media_workflows_to_v2(
        &self,
        project_id: i64,
        cleanup_intent_id: i64,
    ) -> Result<(), AppError> {
        self.orchestrator
            .reset_media_workflows_to_v2(project_id, cleanup_intent_id)
            .await
    }

    pub(crate) async fn complete_claimed_cleanup(
        &self,
        id: i64,
        lease_token: &str,
    ) -> Result<(), AppError> {
        self.orchestrator
            .complete_claimed_cleanup(id, lease_token)
            .await
    }

    pub(crate) async fn complete_claimed_generation_cleanup(
        &self,
        id: i64,
        lease_token: &str,
        outcome: a4n_db::cleanup_outbox::GenerationCleanupOutcome,
    ) -> Result<(), AppError> {
        self.orchestrator
            .complete_claimed_generation_cleanup(id, lease_token, outcome)
            .await
    }

    pub(crate) async fn observe_claimed_generation_reconcile(
        &self,
        id: i64,
        lease_token: &str,
        retry_after_seconds: i64,
    ) -> Result<a4n_db::cleanup_outbox::GenerationCleanupObservationOutcome, AppError> {
        self.orchestrator
            .observe_claimed_generation_reconcile(id, lease_token, retry_after_seconds)
            .await
    }

    pub(crate) async fn schedule_provider_delete_from_reconcile(
        &self,
        id: i64,
        lease_token: &str,
    ) -> Result<a4n_core::models::task::cleanup_outbox::Model, AppError> {
        self.orchestrator
            .schedule_provider_delete_from_reconcile(id, lease_token)
            .await
    }

    pub(crate) async fn renew_claimed_cleanup(
        &self,
        id: i64,
        lease_token: &str,
    ) -> Result<(), AppError> {
        self.orchestrator
            .renew_claimed_cleanup(id, lease_token)
            .await
    }

    pub(crate) async fn retry_claimed_cleanup(
        &self,
        id: i64,
        lease_token: &str,
        error: String,
        retry_after_seconds: i64,
    ) -> Result<(), AppError> {
        self.orchestrator
            .retry_claimed_cleanup(id, lease_token, error, retry_after_seconds)
            .await
    }

    pub(crate) async fn retry_or_fail_claimed_generation_cleanup(
        &self,
        id: i64,
        lease_token: &str,
        error: String,
    ) -> Result<a4n_db::cleanup_outbox::GenerationCleanupRetryOutcome, AppError> {
        self.orchestrator
            .retry_or_fail_claimed_generation_cleanup(id, lease_token, error)
            .await
    }

    pub(crate) async fn unreferenced_media_paths(
        &self,
        paths: &[String],
    ) -> Result<Vec<String>, AppError> {
        self.orchestrator.unreferenced_media_paths(paths).await
    }

    pub(crate) async fn media_path_prefix_has_persisted_reference(
        &self,
        prefix: &str,
    ) -> Result<bool, AppError> {
        self.orchestrator
            .media_path_prefix_has_persisted_reference(prefix)
            .await
    }

    pub(crate) async fn reset_project_generated_data_validated(
        &self,
        project_id: i64,
    ) -> Result<a4n_orchestrator::project::ProjectGeneratedDataResetApplicationResult, AppError>
    {
        self.orchestrator
            .reset_project_generated_data_validated(project_id)
            .await
    }
}
