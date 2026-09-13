use crate::AppState;
use crate::actions::app_error_message;
use a4n_api_contract::portrait::{
    GetPrimaryPortraitInput, ListPortraitsByProjectInput, PortraitSummary, SetPrimaryPortraitInput,
};

pub async fn list_portraits_by_project(
    state: &AppState,
    user_id: i64,
    input: ListPortraitsByProjectInput,
) -> Result<Vec<PortraitSummary>, String> {
    state
        .portrait_record_read_application()
        .list_by_project(user_id, input.project_id)
        .await
        .map_err(app_error_message)
}

pub async fn get_primary_portrait(
    state: &AppState,
    user_id: i64,
    input: GetPrimaryPortraitInput,
) -> Result<Option<PortraitSummary>, String> {
    let portrait = state
        .portrait_record_read_application()
        .primary_by_profile(user_id, input.profile_id)
        .await
        .map_err(app_error_message)?;
    Ok(portrait)
}

pub async fn set_primary_portrait(
    state: &AppState,
    user_id: i64,
    input: SetPrimaryPortraitInput,
) -> Result<(), String> {
    state
        .portrait_record_mutation_application()
        .set_primary(user_id, input.portrait_id)
        .await
        .map_err(app_error_message)
}
