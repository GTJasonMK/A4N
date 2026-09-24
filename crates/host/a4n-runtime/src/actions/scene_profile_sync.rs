pub use a4n_scene_host_contract::{SCENE_PROFILE_SYNC_PROGRESS_TOTAL, SceneProfileSyncProgress};

use a4n_api_contract::profile::SyncSceneProfilesInput;
use a4n_domain::scene_profile::SceneProfileSyncRecordsInput;
use a4n_error::AppError;
use a4n_scene_host_contract::SceneProfileSyncHostPort;
use async_trait::async_trait;

use crate::{rag_ingest::SceneRagSyncTraceSource, state::AppState};

pub(crate) struct RuntimeSceneProfileSyncHostPort {
    state: AppState,
    scene_profile_worker:
        std::sync::Arc<dyn a4n_api_contract::profile::SceneProfileRecordSyncApplicationPort>,
}

impl RuntimeSceneProfileSyncHostPort {
    pub(crate) fn new(state: &AppState) -> Self {
        Self {
            state: state.clone(),
            scene_profile_worker: state.scene_profile_record_sync_application(),
        }
    }
}

#[async_trait]
impl SceneProfileSyncHostPort for RuntimeSceneProfileSyncHostPort {
    async fn replace_chapter_scene_sync_records(
        &self,
        user_id: i64,
        input: SceneProfileSyncRecordsInput,
    ) -> Result<(), AppError> {
        self.scene_profile_worker
            .replace_chapter_scene_sync_records(user_id, input)
            .await
    }

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
        .map_err(AppError::VectorStoreError)
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

pub async fn resume_scene_profile_sync_workflow_run(
    state: &AppState,
    user_id: i64,
    workflow_run_id: i64,
) -> Result<(), String> {
    let prompt_renderer = state;
    let host_port = RuntimeSceneProfileSyncHostPort::new(state);
    state
        .scene_profile_sync_application()
        .resume_workflow_run(user_id, workflow_run_id, prompt_renderer, &host_port)
        .await
        .map_err(|error| error.to_string())
}
