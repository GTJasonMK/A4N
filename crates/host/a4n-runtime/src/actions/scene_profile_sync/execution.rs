use a4n_api_contract::profile::SyncSceneProfilesInput;
use a4n_error::AppError;
use a4n_scene_host_contract::{SceneProfileSyncHostPort, SceneProfileSyncProgress};
use async_trait::async_trait;

use crate::{rag_ingest::SceneRagSyncTraceSource, state::AppState};

pub(crate) struct RuntimeSceneProfileSyncHostPort {
    state: AppState,
}

impl RuntimeSceneProfileSyncHostPort {
    pub(crate) fn new(state: &AppState) -> Self {
        Self {
            state: state.clone(),
        }
    }
}

#[async_trait]
impl SceneProfileSyncHostPort for RuntimeSceneProfileSyncHostPort {
    async fn sync_scene_rag_records(
        &self,
        project_id: i64,
        workflow_run_id: i64,
    ) -> Result<(), AppError> {
        crate::rag_ingest::sync_scene_rag_records(
            &self.state,
            project_id,
            SceneRagSyncTraceSource::WorkflowRun(workflow_run_id),
        )
        .await
        .map_err(|message| AppError::InvalidState {
            current: "scene_profile_sync_rag".to_string(),
            operation: message,
        })
    }
}

pub async fn sync_scene_profiles(
    state: &AppState,
    user_id: i64,
    input: SyncSceneProfilesInput,
    progress: Option<&SceneProfileSyncProgress<'_>>,
) -> Result<(), AppError> {
    let prompt_renderer = state;
    let host_port = RuntimeSceneProfileSyncHostPort::new(state);
    state
        .scene_profile_sync_application()
        .sync(user_id, input, prompt_renderer, &host_port, progress)
        .await
}
