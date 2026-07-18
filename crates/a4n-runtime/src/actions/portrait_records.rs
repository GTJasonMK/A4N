use crate::AppState;
use crate::actions::portrait_generation::{PortraitSummary, to_summary};
use crate::actions::{access, app_error_message};

pub async fn list_portraits_by_project(
    state: &AppState,
    user_id: i64,
    project_id: i64,
) -> Result<Vec<PortraitSummary>, String> {
    access::verify_project_access_string(state, user_id, project_id).await?;
    let portraits = state
        .orchestrator
        .character_portrait_records_by_project(project_id)
        .await
        .map_err(app_error_message)?;
    Ok(portraits.iter().map(to_summary).collect())
}

pub async fn get_primary_portrait(
    state: &AppState,
    user_id: i64,
    profile_id: i64,
) -> Result<Option<PortraitSummary>, String> {
    let project_id = profile_project_id(state, profile_id).await?;
    access::verify_project_access_string(state, user_id, project_id).await?;
    let portrait = state
        .orchestrator
        .primary_character_portrait_record(profile_id)
        .await
        .map_err(app_error_message)?;
    Ok(portrait.as_ref().map(to_summary))
}

pub async fn set_primary_portrait(
    state: &AppState,
    user_id: i64,
    portrait_id: i64,
) -> Result<(), String> {
    portrait_project_id_for_user(state, user_id, portrait_id).await?;
    state
        .orchestrator
        .set_primary_character_portrait_record(portrait_id)
        .await
        .map_err(app_error_message)
}

async fn portrait_project_id(state: &AppState, portrait_id: i64) -> Result<i64, String> {
    state
        .orchestrator
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
        .orchestrator
        .protagonist_profile_project_id(profile_id)
        .await
        .map_err(app_error_message)
}
