use a4n_api_contract::chapter::{ChapterContentSyncApplicationPort, ChapterDerivedCleanupHostPort};
use a4n_domain::project::ChapterDerivedCleanupArtifacts;
use a4n_error::AppError;
use async_trait::async_trait;

use crate::OrchestratorContext;

impl OrchestratorContext {
    pub async fn postprocess_chapter_derived_data_validated(
        &self,
        host_port: &dyn ChapterDerivedCleanupHostPort,
        chapter_id: i64,
    ) -> Result<(), AppError> {
        let chapter = self.chapter_by_id_for_analysis(chapter_id).await?;
        let lock = self.chapter_content_sync_lock(chapter.project_id, chapter.chapter_number);
        let _guard = lock.lock_owned().await;
        let run = self.begin_existing_chapter_content_sync(chapter_id).await?;
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
