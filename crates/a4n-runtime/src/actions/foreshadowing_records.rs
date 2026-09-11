pub use a4n_api_contract::profile::{
    CreateForeshadowingInput, DeleteForeshadowingInput, ForeshadowingSummary,
    ForeshadowingWriteResult, ListForeshadowingInput, UpdateForeshadowingInput,
};

use crate::AppState;

pub async fn list_foreshadowing(
    state: &AppState,
    user_id: i64,
    project_id: i64,
) -> Result<Vec<ForeshadowingSummary>, String> {
    state
        .foreshadowing_application()
        .list(user_id, project_id)
        .await
        .map_err(|error| error.to_string())
}

pub async fn create_foreshadowing(
    state: &AppState,
    user_id: i64,
    input: CreateForeshadowingInput,
) -> Result<ForeshadowingWriteResult, String> {
    state
        .foreshadowing_application()
        .create(user_id, input)
        .await
        .map_err(|error| error.to_string())
}

pub async fn update_foreshadowing(
    state: &AppState,
    user_id: i64,
    input: UpdateForeshadowingInput,
) -> Result<ForeshadowingWriteResult, String> {
    state
        .foreshadowing_application()
        .update(user_id, input)
        .await
        .map_err(|error| error.to_string())
}

pub async fn delete_foreshadowing(
    state: &AppState,
    user_id: i64,
    input: DeleteForeshadowingInput,
) -> Result<ForeshadowingWriteResult, String> {
    state
        .foreshadowing_application()
        .delete(user_id, input)
        .await
        .map_err(|error| error.to_string())
}
