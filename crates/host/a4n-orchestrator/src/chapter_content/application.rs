use std::collections::BTreeSet;

use a4n_api_contract::chapter::{ChapterContentSyncApplicationPort, ChapterDerivedCleanupHostPort};
use a4n_domain::project::ChapterDerivedCleanupArtifacts;
use a4n_error::AppError;
use async_trait::async_trait;

use crate::{OrchestratorContext, project::validate_chapter_generation_number};

impl OrchestratorContext {
    pub async fn preflight_chapter_content_change_validated(
        &self,
        project_id: i64,
        chapter_number: i32,
    ) -> Result<(), AppError> {
        validate_chapter_generation_number(chapter_number).map_err(AppError::InvalidArgument)?;
        let profiles = self.protagonist_profiles_for_project(project_id).await?;
        let snapshot_chapter = chapter_number - 1;
        let snapshot_profile_ids = profiles
            .iter()
            .filter(|profile| {
                a4n_chapter_content_core::chapter_cleanup_requires_snapshot(profile, chapter_number)
            })
            .map(|profile| profile.id)
            .collect::<Vec<_>>();
        if snapshot_profile_ids.is_empty() {
            return Ok(());
        }

        let existing_snapshot_profile_ids = self
            .snapshot_profile_ids_for_chapter(&snapshot_profile_ids, snapshot_chapter)
            .await?
            .into_iter()
            .collect::<BTreeSet<_>>();
        a4n_chapter_content_core::validate_chapter_content_snapshot_preflight(
            &profiles,
            &existing_snapshot_profile_ids,
            chapter_number,
        )
    }

    pub async fn postprocess_chapter_derived_data_validated(
        &self,
        host_port: &dyn ChapterDerivedCleanupHostPort,
        chapter_id: i64,
    ) -> Result<(), AppError> {
        let chapter = self.chapter_by_id_for_analysis(chapter_id).await?;
        let lock = self.chapter_content_sync_lock(chapter.project_id, chapter.chapter_number);
        let _guard = lock.lock_owned().await;
        let run = self
            .begin_existing_chapter_content_sync(chapter_id, |artifacts| {
                host_port.validate_chapter_cleanup_artifacts(artifacts)
            })
            .await?;
        self.settle_chapter_content_sync_run(host_port, &run)
            .await?;
        Ok(())
    }
}

#[async_trait]
impl ChapterContentSyncApplicationPort for OrchestratorContext {
    async fn settle(
        &self,
        host_port: &dyn ChapterDerivedCleanupHostPort,
        workflow_run_id: i64,
    ) -> Result<(), AppError> {
        let run = self.workflow_run_model_by_id(workflow_run_id).await?;
        self.settle_chapter_content_sync_run(host_port, &run)
            .await
            .map(|_| ())
    }

    async fn resume(
        &self,
        host_port: &dyn ChapterDerivedCleanupHostPort,
        workflow_run_id: i64,
    ) -> Result<(), AppError> {
        let run = self.workflow_run_model_by_id(workflow_run_id).await?;
        self.resume_chapter_content_sync_workflow_run_validated(host_port, &run)
            .await
            .map(|_| ())
    }

    async fn postprocess(
        &self,
        host_port: &dyn ChapterDerivedCleanupHostPort,
        chapter_id: i64,
    ) -> Result<(), AppError> {
        self.postprocess_chapter_derived_data_validated(host_port, chapter_id)
            .await
    }

    async fn cleanup_derived_rag(
        &self,
        project_id: i64,
        chapter_id: i64,
        chapter_number: i32,
        artifacts: &ChapterDerivedCleanupArtifacts,
    ) -> Result<(), AppError> {
        self.cleanup_chapter_derived_rag(project_id, chapter_id, chapter_number, artifacts)
            .await
            .map_err(|error| AppError::InvalidState {
                current: "chapter_derived_rag_cleanup".to_string(),
                operation: error,
            })
    }
}
