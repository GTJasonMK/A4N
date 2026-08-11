use a4n_core::error::AppError;
use a4n_core::models::chapter::chapter;
use a4n_orchestrator::{
    chapter_analysis::ChapterAnalysisHostPort, chapter_content::ChapterDerivedCleanupHostPort,
    project::ChapterDerivedCleanupArtifacts, scene_profile::SceneProfileSyncHostPort,
};
use async_trait::async_trait;

use crate::{
    actions::scene_profile_sync::RuntimeSceneProfileSyncHostPort,
    file_utils::deletable_data_file_path,
    rag_ingest::chapter_rag_chunking_mode,
    sidecar_clients::{scope_rag_operation_trace, workflow_rag_operation_trace_id},
    state::AppState,
};

pub(crate) struct RuntimeChapterAnalysisHostPort {
    state: AppState,
    scene_profile: RuntimeSceneProfileSyncHostPort,
}

impl RuntimeChapterAnalysisHostPort {
    pub(crate) fn new(state: &AppState) -> Self {
        Self {
            state: state.clone(),
            scene_profile: RuntimeSceneProfileSyncHostPort::new(state),
        }
    }
}

#[async_trait]
impl ChapterDerivedCleanupHostPort for RuntimeChapterAnalysisHostPort {
    fn validate_chapter_cleanup_artifacts(
        &self,
        artifacts: &ChapterDerivedCleanupArtifacts,
    ) -> Result<(), AppError> {
        for path in &artifacts.media_file_paths {
            deletable_data_file_path(self.state.data_dir(), path, "章节媒体文件").map_err(
                |error| AppError::InvalidState {
                    current: error,
                    operation: "清理章节派生媒体文件".to_string(),
                },
            )?;
        }
        Ok(())
    }

    async fn finish_chapter_derived_cleanup(
        &self,
        project_id: i64,
        chapter_id: i64,
        chapter_number: i32,
        artifacts: ChapterDerivedCleanupArtifacts,
    ) -> Result<(), AppError> {
        crate::workflow::chapter_cleanup::finish_chapter_derived_cleanup(
            &self.state,
            project_id,
            chapter_id,
            chapter_number,
            artifacts,
        )
        .await
        .map_err(host_error)
    }

    async fn sync_chapter_rag_records(
        &self,
        chapter: &chapter::Model,
        workflow_run_id: i64,
    ) -> Result<(), AppError> {
        let trace_id =
            workflow_rag_operation_trace_id(workflow_run_id).map_err(AppError::InvalidArgument)?;
        scope_rag_operation_trace(trace_id, async {
            self.state
                .orchestrator()
                .sync_chapter_rag_records(chapter, chapter_rag_chunking_mode())
                .await
                .map(|_| ())
                .map_err(host_error)
        })
        .await
    }
}

#[async_trait]
impl ChapterAnalysisHostPort for RuntimeChapterAnalysisHostPort {
    fn scene_profile_sync_host_port(&self) -> &dyn SceneProfileSyncHostPort {
        &self.scene_profile
    }
}

fn host_error(error: String) -> AppError {
    AppError::InvalidState {
        current: "chapter_analysis_host".to_string(),
        operation: error,
    }
}
