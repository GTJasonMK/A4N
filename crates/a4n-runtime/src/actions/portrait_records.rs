use crate::AppState;
use crate::actions::portrait_generation::{PortraitSummary, to_summary};
use crate::actions::{access, app_error_message};
use a4n_api_contract::portrait::{
    GetPrimaryPortraitInput, ListPortraitsByProjectInput, SetPrimaryPortraitInput,
};

pub async fn list_portraits_by_project(
    state: &AppState,
    user_id: i64,
    input: ListPortraitsByProjectInput,
) -> Result<Vec<PortraitSummary>, String> {
    access::verify_project_access_string(state, user_id, input.project_id).await?;
    let portraits = state
        .orchestrator()
        .character_portrait_records_by_project(input.project_id)
        .await
        .map_err(app_error_message)?;
    portraits
        .iter()
        .map(to_summary)
        .collect::<Result<Vec<_>, _>>()
        .map_err(app_error_message)
}

pub async fn get_primary_portrait(
    state: &AppState,
    user_id: i64,
    input: GetPrimaryPortraitInput,
) -> Result<Option<PortraitSummary>, String> {
    let project_id = profile_project_id(state, input.profile_id).await?;
    access::verify_project_access_string(state, user_id, project_id).await?;
    let portrait = state
        .orchestrator()
        .primary_character_portrait_record(input.profile_id)
        .await
        .map_err(app_error_message)?;
    portrait
        .as_ref()
        .map(to_summary)
        .transpose()
        .map_err(app_error_message)
}

pub async fn set_primary_portrait(
    state: &AppState,
    user_id: i64,
    input: SetPrimaryPortraitInput,
) -> Result<(), String> {
    portrait_project_id_for_user(state, user_id, input.portrait_id).await?;
    state
        .orchestrator()
        .set_primary_character_portrait_record(input.portrait_id)
        .await
        .map_err(app_error_message)
}

async fn portrait_project_id(state: &AppState, portrait_id: i64) -> Result<i64, String> {
    state
        .orchestrator()
        .character_portrait_project_id(portrait_id)
        .await
        .map_err(app_error_message)
}

pub(crate) async fn portrait_project_id_for_user(
    state: &AppState,
    user_id: i64,
    portrait_id: i64,
) -> Result<i64, String> {
    let project_id = portrait_project_id(state, portrait_id).await?;
    access::verify_project_editor_string(state, user_id, project_id).await?;
    Ok(project_id)
}

async fn profile_project_id(state: &AppState, profile_id: i64) -> Result<i64, String> {
    state
        .orchestrator()
        .protagonist_profile_project_id(profile_id)
        .await
        .map_err(app_error_message)
}
