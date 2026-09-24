pub use a4n_api_contract::profile::{
    CreateForeshadowingInput, DeleteForeshadowingInput, ForeshadowingSummary,
    ForeshadowingWriteResult, ListForeshadowingInput, UpdateForeshadowingInput,
};

use a4n_api_contract::profile::ForeshadowingRagSyncHostPort;
use a4n_error::AppError;
use async_trait::async_trait;

use crate::AppState;

struct RuntimeForeshadowingRagSyncHostPort<'a>(&'a AppState);

#[async_trait]
impl ForeshadowingRagSyncHostPort for RuntimeForeshadowingRagSyncHostPort<'_> {
    async fn sync_record(&self, project_id: i64, foreshadowing_id: i64) -> Result<(), AppError> {
        self.0
            .rag_ingest()
            .sync_foreshadowing_record(project_id, foreshadowing_id)
            .await
    }

    async fn delete_record(&self, project_id: i64, foreshadowing_id: i64) -> Result<(), AppError> {
        self.0
            .rag_ingest()
            .delete_foreshadowing_rag_record(project_id, foreshadowing_id)
            .await
    }
}

pub async fn list_foreshadowing(
    state: &AppState,
    user_id: i64,
    project_id: i64,
) -> Result<Vec<ForeshadowingSummary>, String> {
    state
        .foreshadowing_read_application()
        .list(user_id, project_id)
        .await
        .map_err(|error| error.to_string())
}

pub async fn create_foreshadowing(
    state: &AppState,
    user_id: i64,
    input: CreateForeshadowingInput,
) -> Result<ForeshadowingWriteResult, String> {
    let host_port = RuntimeForeshadowingRagSyncHostPort(state);
    state
        .foreshadowing_mutation_application()
        .create(user_id, &host_port, input)
        .await
        .map_err(|error| error.to_string())
}

pub async fn update_foreshadowing(
    state: &AppState,
    user_id: i64,
    input: UpdateForeshadowingInput,
) -> Result<ForeshadowingWriteResult, String> {
    let host_port = RuntimeForeshadowingRagSyncHostPort(state);
    state
        .foreshadowing_mutation_application()
        .update(user_id, &host_port, input)
        .await
        .map_err(|error| error.to_string())
}

pub async fn delete_foreshadowing(
    state: &AppState,
    user_id: i64,
    input: DeleteForeshadowingInput,
) -> Result<ForeshadowingWriteResult, String> {
    let host_port = RuntimeForeshadowingRagSyncHostPort(state);
    state
        .foreshadowing_mutation_application()
        .delete(user_id, &host_port, input)
        .await
        .map_err(|error| error.to_string())
}
