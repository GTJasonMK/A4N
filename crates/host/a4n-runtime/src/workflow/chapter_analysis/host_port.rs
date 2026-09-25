use a4n_api_contract::chapter::ChapterDerivedCleanupHostPort;
use a4n_api_contract::character::{
    CharacterAttributeRagSyncHostPort, CharacterProfileDeletionHostPort,
    CharacterProfileTimelineMutationHostPort,
};
use a4n_chapter_analysis_host_contract::ChapterAnalysisHostPort;
use a4n_domain::project::ChapterDerivedCleanupArtifacts;
use a4n_error::AppError;
use a4n_scene_host_contract::SceneProfileSyncHostPort;
use async_trait::async_trait;

use crate::{
    actions::scene_profile_sync::RuntimeSceneProfileSyncHostPort,
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
impl CharacterAttributeRagSyncHostPort for RuntimeChapterAnalysisHostPort {
    async fn sync_attribute_event(&self, project_id: i64, event_id: i64) -> Result<(), AppError> {
        self.state
            .rag_ingest()
            .sync_character_attribute_event(project_id, event_id)
            .await
            .map_err(|error| AppError::Internal(anyhow::anyhow!(error.to_string())))
    }
}

#[async_trait]
impl CharacterProfileDeletionHostPort for RuntimeChapterAnalysisHostPort {
    async fn sync_profile_rag(&self, project_id: i64) -> Result<(), AppError> {
        crate::rag_ingest::sync_protagonist_rag_records(&self.state, project_id)
            .await
            .map_err(|error| AppError::Internal(anyhow::anyhow!(error)))
    }
}

#[async_trait]
impl CharacterProfileTimelineMutationHostPort for RuntimeChapterAnalysisHostPort {
    async fn delete_timeline_rag(
        &self,
        project_id: i64,
        state_index_ids: Vec<i64>,
        attribute_event_ids: Vec<i64>,
    ) -> Result<(usize, usize), AppError> {
        self.state
            .rag_ingest()
            .delete_character_timeline_rag_records(project_id, state_index_ids, attribute_event_ids)
            .await
    }
}

#[async_trait]
impl ChapterDerivedCleanupHostPort for RuntimeChapterAnalysisHostPort {
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
        project_id: i64,
        chapter_id: i64,
        workflow_run_id: i64,
    ) -> Result<(), AppError> {
        let trace_id =
            workflow_rag_operation_trace_id(workflow_run_id).map_err(AppError::InvalidArgument)?;
        scope_rag_operation_trace(trace_id, async {
            self.state
                .rag_ingest()
                .sync_chapter_records(project_id, chapter_id, chapter_rag_chunking_mode())
                .await
                .map(|_| ())
        })
        .await
    }
}

#[async_trait]
impl ChapterAnalysisHostPort for RuntimeChapterAnalysisHostPort {
    fn scene_profile_sync_host_port(&self) -> &dyn SceneProfileSyncHostPort {
        &self.scene_profile
    }

    async fn sync_fact_material_rag_records(
        &self,
        project_id: i64,
        workflow_run_id: i64,
        blueprint_changed: bool,
        characters_changed: bool,
    ) -> Result<(), AppError> {
        let trace_id =
            workflow_rag_operation_trace_id(workflow_run_id).map_err(AppError::InvalidArgument)?;
        scope_rag_operation_trace(trace_id, async {
            if blueprint_changed {
                crate::rag_ingest::sync_blueprint_material_rag_records(&self.state, project_id)
                    .await
                    .map_err(host_error)?;
            } else if characters_changed {
                crate::rag_ingest::sync_character_continuity_rag_records(&self.state, project_id)
                    .await
                    .map_err(host_error)?;
            }
            Ok(())
        })
        .await
    }
}

fn host_error(error: String) -> AppError {
    AppError::InvalidState {
        current: "chapter_analysis_host".to_string(),
        operation: error,
    }
}
