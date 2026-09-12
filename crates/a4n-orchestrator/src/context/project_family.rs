use a4n_db::{DbHandle, project::MangaCharacterPortraitRefSummary};
use a4n_db_models::models::project::novel_project;
use a4n_domain::project::{ProjectSourceMode, ProjectStatus};
use a4n_error::AppError;
use a4n_event::InternalEvent;
use chrono::{Duration, Utc};

use super::OrchestratorContext;
use crate::collaboration::ProjectAccessRole;
use crate::project::{
    ChapterMangaSourceSummary, ProjectFamilyContext, ProjectFamilyExportRows,
    ProjectFamilyExportSelection, ProjectFamilyImportPackage, ProjectFamilyMediaFileImporter,
    ProjectGeneratedDataResetApplicationResult, SourceSnapshotChapterSummary,
    ensure_positive_project_id, project_family_error_to_app_error,
    project_family_export_error_to_app_error, project_family_import_error_to_app_error,
    project_mutation_error_to_app_error, project_reset_error_to_app_error,
};

#[derive(Clone)]
pub(super) struct ProjectFamilyApplication {
    database: DbHandle,
}

impl ProjectFamilyApplication {
    pub(super) fn new(database: DbHandle) -> Self {
        Self { database }
    }

    async fn protagonist_profile_project_id(&self, profile_id: i64) -> Result<i64, AppError> {
        a4n_db::project::protagonist_profile_project_id(self.database.connection(), profile_id)
            .await
            .map_err(project_family_error_to_app_error)
    }

    async fn primary_material_image_ids_for_projects(
        &self,
        project_ids: &[i64],
    ) -> Result<Vec<i64>, AppError> {
        a4n_db::project::primary_material_image_ids_for_projects(
            self.database.connection(),
            project_ids,
        )
        .await
        .map_err(project_family_error_to_app_error)
    }

    async fn derived_project_context(
        &self,
        project_id: i64,
    ) -> Result<ProjectFamilyContext, AppError> {
        a4n_db::project::derived_project_family_context(self.database.connection(), project_id)
            .await
            .map_err(project_family_error_to_app_error)
    }

    async fn project_or_derived_access_ids(
        &self,
        project_id: i64,
    ) -> Result<(i64, Option<i64>), AppError> {
        a4n_db::project::project_or_derived_family_access_ids(
            self.database.connection(),
            project_id,
        )
        .await
        .map_err(project_family_error_to_app_error)
    }

    async fn manga_character_portrait_refs(
        &self,
        project_id: i64,
    ) -> Result<Vec<MangaCharacterPortraitRefSummary>, AppError> {
        a4n_db::project::manga_character_portrait_refs(self.database.connection(), project_id)
            .await
            .map_err(project_family_error_to_app_error)
    }

    async fn chapter_manga_source_by_id(
        &self,
        chapter_id: i64,
        project_id: i64,
        require_content: bool,
    ) -> Result<ChapterMangaSourceSummary, AppError> {
        a4n_db::project::chapter_manga_source_by_id(
            self.database.connection(),
            chapter_id,
            project_id,
            require_content,
        )
        .await
        .map_err(project_family_error_to_app_error)
    }

    async fn chapter_manga_source_by_number(
        &self,
        project_id: i64,
        chapter_number: i32,
        require_content: bool,
    ) -> Result<ChapterMangaSourceSummary, AppError> {
        a4n_db::project::chapter_manga_source_by_number(
            self.database.connection(),
            project_id,
            chapter_number,
            require_content,
        )
        .await
        .map_err(project_family_error_to_app_error)
    }

    async fn chapter_manga_sources_by_ids(
        &self,
        project_id: i64,
        chapter_ids: &[i64],
        require_content: bool,
    ) -> Result<Vec<ChapterMangaSourceSummary>, AppError> {
        a4n_db::project::chapter_manga_sources_by_ids(
            self.database.connection(),
            project_id,
            chapter_ids,
            require_content,
        )
        .await
        .map_err(project_family_error_to_app_error)
    }

    async fn source_snapshot_chapters(
        &self,
        project_id: i64,
    ) -> Result<Vec<SourceSnapshotChapterSummary>, AppError> {
        a4n_db::project::source_snapshot_chapters(self.database.connection(), project_id)
            .await
            .map_err(project_mutation_error_to_app_error)
    }

    pub(super) async fn export_selection(
        &self,
        root_project_id: i64,
        include_derived_projects: bool,
    ) -> Result<ProjectFamilyExportSelection, AppError> {
        a4n_db::project::project_family_export_selection(
            self.database.connection(),
            root_project_id,
            include_derived_projects,
        )
        .await
        .map_err(project_family_export_error_to_app_error)
    }

    pub(super) async fn derived_export_selection(
        &self,
        derived_project_id: i64,
    ) -> Result<ProjectFamilyExportSelection, AppError> {
        a4n_db::project::derived_project_export_selection(
            self.database.connection(),
            derived_project_id,
        )
        .await
        .map_err(project_family_export_error_to_app_error)
    }

    pub(super) async fn export_rows(
        &self,
        root_project_id: i64,
        derived_project_ids: &[i64],
        derived_source_snapshot_ids: &[i64],
    ) -> Result<ProjectFamilyExportRows, AppError> {
        a4n_db::project::project_family_export_rows(
            self.database.connection(),
            root_project_id,
            derived_project_ids,
            derived_source_snapshot_ids,
        )
        .await
        .map_err(project_family_export_error_to_app_error)
    }

    pub(super) async fn import_package_rows<Importer>(
        &self,
        user_id: i64,
        package: &ProjectFamilyImportPackage,
        media_importer: &mut Importer,
    ) -> Result<Vec<novel_project::Model>, AppError>
    where
        Importer: ProjectFamilyMediaFileImporter,
    {
        a4n_db::project_family_import::import_project_family_package_rows(
            self.database.connection(),
            user_id,
            package,
            media_importer,
        )
        .await
        .map_err(project_family_import_error_to_app_error)
    }

    async fn delete_project_cascade(&self, project_id: i64) -> Result<(), AppError> {
        a4n_db::project_delete::delete_project_cascade_record(
            self.database.connection(),
            project_id,
        )
        .await
        .map_err(crate::project_delete_error_to_app_error)
    }

    async fn claim_project_deletion_cleanup(
        &self,
        project_id: Option<i64>,
    ) -> Result<Option<a4n_cleanup_contract::ProjectDeletionCleanupClaim>, AppError> {
        a4n_db::cleanup_outbox::claim_project_deletion_cleanup(
            self.database.connection(),
            project_id,
        )
        .await
        .map(|claim| claim.map(Into::into))
        .map_err(AppError::Database)
    }

    async fn claim_media_workflow_v2_reset(
        &self,
        project_id: Option<i64>,
    ) -> Result<Option<a4n_cleanup_contract::MediaWorkflowV2ResetClaim>, AppError> {
        a4n_db::cleanup_outbox::claim_media_workflow_v2_reset(
            self.database.connection(),
            project_id,
        )
        .await
        .map(|claim| claim.map(Into::into))
        .map_err(AppError::Database)
    }

    async fn insert_file_artifact_cleanup_intent(
        &self,
        project_id: i64,
        dedupe_key: String,
        paths: Vec<String>,
    ) -> Result<(), AppError> {
        a4n_db::cleanup_outbox::insert_file_artifact_cleanup_intent(
            self.database.connection(),
            dedupe_key,
            a4n_db::cleanup_outbox::FileArtifactCleanupPayload::new(project_id, paths),
        )
        .await
        .map(|_| ())
        .map_err(AppError::Database)
    }

    async fn claim_file_artifact_cleanup(
        &self,
        project_id: Option<i64>,
    ) -> Result<Option<a4n_cleanup_contract::FileArtifactCleanupClaim>, AppError> {
        a4n_db::cleanup_outbox::claim_file_artifact_cleanup(self.database.connection(), project_id)
            .await
            .map(|claim| claim.map(Into::into))
            .map_err(AppError::Database)
    }

    async fn claim_generation_cleanup(
        &self,
        project_id: Option<i64>,
    ) -> Result<Option<a4n_cleanup_contract::GenerationCleanupClaim>, AppError> {
        a4n_db::cleanup_outbox::claim_generation_cleanup(self.database.connection(), project_id)
            .await
            .map(|claim| claim.map(Into::into))
            .map_err(AppError::Database)
    }

    async fn reset_media_workflows_to_v2(
        &self,
        project_id: i64,
        cleanup_intent_id: i64,
    ) -> Result<(), AppError> {
        a4n_db::media_workflow_reset::reset_media_workflows_to_v2(
            self.database.connection(),
            project_id,
            cleanup_intent_id,
        )
        .await
        .map_err(AppError::Database)
    }

    async fn media_workflow_v2_reset_pending(&self, project_id: i64) -> Result<bool, AppError> {
        a4n_db::cleanup_outbox::media_workflow_v2_reset_pending(
            self.database.connection(),
            project_id,
        )
        .await
        .map_err(AppError::Database)
    }

    async fn project_ids_with_incomplete_cleanup(&self) -> Result<Vec<i64>, AppError> {
        a4n_db::cleanup_outbox::project_ids_with_incomplete_cleanup(self.database.connection())
            .await
            .map_err(AppError::Database)
    }

    async fn purge_completed_cleanup_audits(&self, limit: usize) -> Result<u64, AppError> {
        a4n_db::cleanup_outbox::purge_completed_cleanup_audits(self.database.connection(), limit)
            .await
            .map_err(AppError::Database)
    }

    async fn complete_claimed_cleanup(&self, id: i64, lease_token: &str) -> Result<(), AppError> {
        a4n_db::cleanup_outbox::complete_claimed_cleanup(
            self.database.connection(),
            id,
            lease_token,
        )
        .await
        .map_err(AppError::Database)
    }

    async fn complete_claimed_generation_cleanup(
        &self,
        id: i64,
        lease_token: &str,
        outcome: a4n_cleanup_contract::GenerationCleanupOutcome,
    ) -> Result<(), AppError> {
        a4n_db::cleanup_outbox::complete_claimed_generation_cleanup(
            self.database.connection(),
            id,
            lease_token,
            outcome,
        )
        .await
        .map_err(AppError::Database)
    }

    async fn observe_claimed_generation_reconcile(
        &self,
        id: i64,
        lease_token: &str,
        retry_after_seconds: i64,
    ) -> Result<a4n_cleanup_contract::GenerationCleanupObservationOutcome, AppError> {
        a4n_db::cleanup_outbox::observe_claimed_generation_reconcile(
            self.database.connection(),
            id,
            lease_token,
            Duration::seconds(retry_after_seconds),
        )
        .await
        .map_err(AppError::Database)
    }

    async fn schedule_provider_delete_from_reconcile(
        &self,
        id: i64,
        lease_token: &str,
    ) -> Result<(), AppError> {
        a4n_db::cleanup_outbox::schedule_provider_delete_from_reconcile(
            self.database.connection(),
            id,
            lease_token,
        )
        .await
        .map(|_| ())
        .map_err(AppError::Database)
    }

    async fn renew_claimed_cleanup(&self, id: i64, lease_token: &str) -> Result<(), AppError> {
        a4n_db::cleanup_outbox::renew_claimed_cleanup(self.database.connection(), id, lease_token)
            .await
            .map_err(AppError::Database)
    }

    async fn retry_claimed_cleanup(
        &self,
        id: i64,
        lease_token: &str,
        error: String,
        retry_after_seconds: i64,
    ) -> Result<(), AppError> {
        a4n_db::cleanup_outbox::retry_claimed_cleanup(
            self.database.connection(),
            id,
            lease_token,
            error,
            Duration::seconds(retry_after_seconds),
        )
        .await
        .map_err(AppError::Database)
    }

    async fn retry_or_fail_claimed_generation_cleanup(
        &self,
        id: i64,
        lease_token: &str,
        error: String,
    ) -> Result<a4n_cleanup_contract::GenerationCleanupRetryOutcome, AppError> {
        a4n_db::cleanup_outbox::retry_or_fail_claimed_generation_cleanup(
            self.database.connection(),
            id,
            lease_token,
            error,
        )
        .await
        .map_err(AppError::Database)
    }

    async fn unreferenced_media_paths(&self, paths: &[String]) -> Result<Vec<String>, AppError> {
        a4n_db::media::filter_unreferenced_media_paths(self.database.connection(), paths)
            .await
            .map_err(AppError::Database)
    }

    async fn media_path_prefix_has_persisted_reference(
        &self,
        prefix: &str,
    ) -> Result<bool, AppError> {
        a4n_db::media::media_path_prefix_has_persisted_reference(self.database.connection(), prefix)
            .await
            .map_err(AppError::Database)
    }

    async fn reset_project_generated_data(
        &self,
        project_id: i64,
    ) -> Result<a4n_domain::project_reset::ProjectGeneratedDataResetOutcome, AppError> {
        a4n_db::project_reset::reset_project_generated_data_record(
            self.database.connection(),
            project_id,
        )
        .await
        .map_err(project_reset_error_to_app_error)
    }

    pub(crate) async fn reset_freeform_video_generated_data_records(
        &self,
        project_id: i64,
    ) -> Result<a4n_domain::project_reset::FreeformVideoGeneratedDataResetResult, AppError> {
        a4n_db::project::reset_freeform_video_generated_data_records(
            self.database.connection(),
            project_id,
        )
        .await
        .map_err(crate::freeform_generated_reset_error_to_app_error)
    }
}

impl OrchestratorContext {
    pub(crate) async fn verify_project_or_derived_family_access_validated(
        &self,
        user_id: i64,
        project_id: i64,
    ) -> Result<(), AppError> {
        let (project_id, root_project_id) = self
            .project_family
            .project_or_derived_access_ids(project_id)
            .await?;
        if let Some(root_project_id) = root_project_id {
            return self
                .verify_project_permissions(
                    &[project_id, root_project_id],
                    user_id,
                    ProjectAccessRole::Viewer,
                )
                .await;
        }
        self.verify_project_permission(project_id, user_id, ProjectAccessRole::Viewer)
            .await
            .map(|_| ())
    }

    pub(crate) async fn verify_project_or_derived_family_editor_validated(
        &self,
        user_id: i64,
        project_id: i64,
    ) -> Result<(), AppError> {
        let (project_id, root_project_id) = self
            .project_family
            .project_or_derived_access_ids(project_id)
            .await?;
        if let Some(root_project_id) = root_project_id {
            return self
                .verify_project_permissions(
                    &[project_id, root_project_id],
                    user_id,
                    ProjectAccessRole::Editor,
                )
                .await;
        }
        self.verify_project_permission(project_id, user_id, ProjectAccessRole::Editor)
            .await
            .map(|_| ())
    }

    pub(crate) async fn derived_project_family_editor_context_validated(
        &self,
        user_id: i64,
        project_id: i64,
    ) -> Result<ProjectFamilyContext, AppError> {
        let context = self.derived_project_family_context(project_id).await?;
        let (project_result, root_project_result) = futures::join!(
            self.verify_project_permission(context.project.id, user_id, ProjectAccessRole::Editor),
            self.verify_project_permission(
                context.root_project.id,
                user_id,
                ProjectAccessRole::Editor
            ),
        );
        project_result?;
        root_project_result?;
        Ok(context)
    }

    pub(crate) async fn protagonist_profile_project_id(
        &self,
        profile_id: i64,
    ) -> Result<i64, AppError> {
        self.project_family
            .protagonist_profile_project_id(profile_id)
            .await
    }

    pub(crate) async fn primary_material_image_ids_for_projects(
        &self,
        project_ids: &[i64],
    ) -> Result<Vec<i64>, AppError> {
        self.project_family
            .primary_material_image_ids_for_projects(project_ids)
            .await
    }

    pub(crate) async fn derived_project_family_context(
        &self,
        project_id: i64,
    ) -> Result<ProjectFamilyContext, AppError> {
        self.project_family
            .derived_project_context(project_id)
            .await
    }

    pub(crate) async fn manga_character_portrait_refs(
        &self,
        project_id: i64,
    ) -> Result<Vec<MangaCharacterPortraitRefSummary>, AppError> {
        self.project_family
            .manga_character_portrait_refs(project_id)
            .await
    }

    pub(crate) async fn chapter_manga_source_by_id(
        &self,
        chapter_id: i64,
        project_id: i64,
        require_content: bool,
    ) -> Result<ChapterMangaSourceSummary, AppError> {
        self.project_family
            .chapter_manga_source_by_id(chapter_id, project_id, require_content)
            .await
    }

    pub(crate) async fn chapter_manga_source_by_number(
        &self,
        project_id: i64,
        chapter_number: i32,
        require_content: bool,
    ) -> Result<ChapterMangaSourceSummary, AppError> {
        self.project_family
            .chapter_manga_source_by_number(project_id, chapter_number, require_content)
            .await
    }

    pub(crate) async fn chapter_manga_sources_by_ids(
        &self,
        project_id: i64,
        chapter_ids: &[i64],
        require_content: bool,
    ) -> Result<Vec<ChapterMangaSourceSummary>, AppError> {
        self.project_family
            .chapter_manga_sources_by_ids(project_id, chapter_ids, require_content)
            .await
    }

    pub(crate) async fn source_snapshot_chapters(
        &self,
        project_id: i64,
    ) -> Result<Vec<SourceSnapshotChapterSummary>, AppError> {
        self.project_family
            .source_snapshot_chapters(project_id)
            .await
    }

    /// 校验项目 ID 后级联删除项目记录。
    pub(crate) async fn delete_project_cascade_validated(
        &self,
        project_id: i64,
    ) -> Result<(), AppError> {
        ensure_positive_project_id(project_id)?;
        self.project_family.delete_project_cascade(project_id).await
    }

    pub(crate) async fn claim_project_deletion_cleanup(
        &self,
        project_id: Option<i64>,
    ) -> Result<Option<a4n_cleanup_contract::ProjectDeletionCleanupClaim>, AppError> {
        self.project_family
            .claim_project_deletion_cleanup(project_id)
            .await
    }

    pub(crate) async fn claim_media_workflow_v2_reset(
        &self,
        project_id: Option<i64>,
    ) -> Result<Option<a4n_cleanup_contract::MediaWorkflowV2ResetClaim>, AppError> {
        self.project_family
            .claim_media_workflow_v2_reset(project_id)
            .await
    }

    pub(crate) async fn insert_file_artifact_cleanup_intent(
        &self,
        project_id: i64,
        dedupe_key: String,
        paths: Vec<String>,
    ) -> Result<(), AppError> {
        self.project_family
            .insert_file_artifact_cleanup_intent(project_id, dedupe_key, paths)
            .await
    }

    pub(crate) async fn claim_file_artifact_cleanup(
        &self,
        project_id: Option<i64>,
    ) -> Result<Option<a4n_cleanup_contract::FileArtifactCleanupClaim>, AppError> {
        self.project_family
            .claim_file_artifact_cleanup(project_id)
            .await
    }

    pub(crate) async fn claim_generation_cleanup(
        &self,
        project_id: Option<i64>,
    ) -> Result<Option<a4n_cleanup_contract::GenerationCleanupClaim>, AppError> {
        self.project_family
            .claim_generation_cleanup(project_id)
            .await
    }

    pub(crate) async fn project_ids_with_incomplete_cleanup(&self) -> Result<Vec<i64>, AppError> {
        self.project_family
            .project_ids_with_incomplete_cleanup()
            .await
    }

    pub(crate) async fn purge_completed_cleanup_audits(
        &self,
        limit: usize,
    ) -> Result<u64, AppError> {
        self.project_family
            .purge_completed_cleanup_audits(limit)
            .await
    }

    pub(crate) async fn reset_media_workflows_to_v2(
        &self,
        project_id: i64,
        cleanup_intent_id: i64,
    ) -> Result<(), AppError> {
        self.project_family
            .reset_media_workflows_to_v2(project_id, cleanup_intent_id)
            .await
    }

    pub(crate) async fn ensure_media_workflow_v2_ready(
        &self,
        project_id: i64,
    ) -> Result<(), AppError> {
        if self
            .project_family
            .media_workflow_v2_reset_pending(project_id)
            .await?
        {
            return Err(AppError::InvalidState {
                current: "media_workflow_contract_v2_reset_pending".to_string(),
                operation: "启动媒体生成流水线".to_string(),
            });
        }
        Ok(())
    }

    pub(crate) async fn complete_claimed_cleanup(
        &self,
        id: i64,
        lease_token: &str,
    ) -> Result<(), AppError> {
        self.project_family
            .complete_claimed_cleanup(id, lease_token)
            .await
    }

    pub(crate) async fn complete_claimed_generation_cleanup(
        &self,
        id: i64,
        lease_token: &str,
        outcome: a4n_cleanup_contract::GenerationCleanupOutcome,
    ) -> Result<(), AppError> {
        self.project_family
            .complete_claimed_generation_cleanup(id, lease_token, outcome)
            .await
    }

    pub(crate) async fn observe_claimed_generation_reconcile(
        &self,
        id: i64,
        lease_token: &str,
        retry_after_seconds: i64,
    ) -> Result<a4n_cleanup_contract::GenerationCleanupObservationOutcome, AppError> {
        self.project_family
            .observe_claimed_generation_reconcile(id, lease_token, retry_after_seconds)
            .await
    }

    pub(crate) async fn schedule_provider_delete_from_reconcile(
        &self,
        id: i64,
        lease_token: &str,
    ) -> Result<(), AppError> {
        self.project_family
            .schedule_provider_delete_from_reconcile(id, lease_token)
            .await
    }

    pub(crate) async fn renew_claimed_cleanup(
        &self,
        id: i64,
        lease_token: &str,
    ) -> Result<(), AppError> {
        self.project_family
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
        self.project_family
            .retry_claimed_cleanup(id, lease_token, error, retry_after_seconds)
            .await
    }

    pub(crate) async fn retry_or_fail_claimed_generation_cleanup(
        &self,
        id: i64,
        lease_token: &str,
        error: String,
    ) -> Result<a4n_cleanup_contract::GenerationCleanupRetryOutcome, AppError> {
        self.project_family
            .retry_or_fail_claimed_generation_cleanup(id, lease_token, error)
            .await
    }

    pub(crate) async fn unreferenced_media_paths(
        &self,
        paths: &[String],
    ) -> Result<Vec<String>, AppError> {
        self.project_family.unreferenced_media_paths(paths).await
    }

    pub(crate) async fn media_path_prefix_has_persisted_reference(
        &self,
        prefix: &str,
    ) -> Result<bool, AppError> {
        self.project_family
            .media_path_prefix_has_persisted_reference(prefix)
            .await
    }

    /// 按项目类型路由生成数据重置（标准 / freeform），含状态变更事件。
    pub(crate) async fn reset_project_generated_data_validated(
        &self,
        project_id: i64,
    ) -> Result<ProjectGeneratedDataResetApplicationResult, AppError> {
        ensure_positive_project_id(project_id)?;
        let project = self
            .project_by_id(project_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("项目不存在: {project_id}")))?;
        if project
            .source_mode()
            .map_err(|error| AppError::InvalidArgument(format!("项目 {project_id}: {error}")))?
            == ProjectSourceMode::Freeform
        {
            let result = self
                .project_family
                .reset_freeform_video_generated_data_records(project_id)
                .await?;
            return Ok(ProjectGeneratedDataResetApplicationResult::Freeform { result });
        }
        let outcome = self
            .project_family
            .reset_project_generated_data(project_id)
            .await?;
        if outcome.previous_status != ProjectStatus::Draft {
            self.publish_internal_event(InternalEvent::ProjectStatusChanged {
                project_id,
                from: outcome.previous_status,
                to: ProjectStatus::Draft,
                timestamp: Utc::now().naive_utc(),
            })?;
        }
        Ok(ProjectGeneratedDataResetApplicationResult::Standard(
            outcome,
        ))
    }
}

#[async_trait::async_trait]
impl crate::cleanup_outbox::CleanupOutboxApplicationPort for OrchestratorContext {
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

    async fn claim_media_workflow_v2_reset(
        &self,
        project_id: Option<i64>,
    ) -> Result<Option<a4n_cleanup_contract::MediaWorkflowV2ResetClaim>, AppError> {
        OrchestratorContext::claim_media_workflow_v2_reset(self, project_id).await
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

    async fn reset_media_workflows_to_v2(
        &self,
        project_id: i64,
        cleanup_intent_id: i64,
    ) -> Result<(), AppError> {
        OrchestratorContext::reset_media_workflows_to_v2(self, project_id, cleanup_intent_id).await
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
