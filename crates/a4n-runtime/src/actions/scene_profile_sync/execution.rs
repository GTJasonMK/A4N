mod scope;

use a4n_core::error::AppError;
use a4n_orchestrator::scene_profile::{SceneProfileSyncHostPort, SceneProfileSyncProgress};
use async_trait::async_trait;
use serde_json::Value;

use crate::{
    prompt_renderer::RuntimePromptRenderer,
    rag_ingest::SceneRagSyncTraceSource,
    sidecar_clients::{scope_rag_operation_trace, workflow_rag_operation_trace_id},
    state::AppState,
};

pub use scope::scene_profile_sync_task_scope;

pub(crate) struct RuntimeSceneProfileSyncHostPort<'a> {
    state: &'a AppState,
}

impl<'a> RuntimeSceneProfileSyncHostPort<'a> {
    pub(crate) fn new(state: &'a AppState) -> Self {
        Self { state }
    }
}

#[async_trait]
impl SceneProfileSyncHostPort for RuntimeSceneProfileSyncHostPort<'_> {
    async fn sync_scene_rag_records(
        &self,
        project_id: i64,
        workflow_run_id: i64,
    ) -> Result<Value, AppError> {
        let trace_id =
            workflow_rag_operation_trace_id(workflow_run_id).map_err(AppError::InvalidArgument)?;
        scope_rag_operation_trace(trace_id, async {
            crate::rag_ingest::sync_scene_rag_records(
                self.state,
                project_id,
                SceneRagSyncTraceSource::WorkflowRun(workflow_run_id),
            )
            .await
            .map_err(|message| AppError::InvalidState {
                current: "scene_profile_sync_rag".to_string(),
                operation: message,
            })
        })
        .await
    }
}

pub async fn sync_scene_profiles_for_chapters(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    chapter_number: Option<i32>,
    progress: Option<&SceneProfileSyncProgress<'_>>,
) -> Result<Value, String> {
    scene_profile_sync_task_scope(state, user_id, project_id).await?;
    let prompt_renderer = RuntimePromptRenderer::new(state);
    let host_port = RuntimeSceneProfileSyncHostPort::new(state);
    state
        .scene_profile()
        .sync_scene_profiles_for_chapters_validated(
            user_id,
            project_id,
            chapter_number,
            &prompt_renderer,
            &host_port,
            progress,
        )
        .await
        .map_err(|error| error.to_string())
}
