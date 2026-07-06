use crate::AppState;
use crate::actions::access;
use crate::actions::portrait_generation::{PortraitSummary, to_summary};
use a4n_core::models::character::{character_portrait, protagonist_profile};
use sea_orm::EntityTrait;

pub async fn list_portraits_by_project(
    state: &AppState,
    user_id: i64,
    project_id: i64,
) -> Result<Vec<PortraitSummary>, String> {
    access::verify_project_access_string(state, user_id, project_id).await?;
    let portraits = state
        .portrait
        .list_by_project(project_id)
        .await
        .map_err(|e| e.to_string())?;
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
        .portrait
        .primary(profile_id)
        .await
        .map_err(|e| e.to_string())?;
    Ok(portrait.as_ref().map(to_summary))
}

pub async fn set_primary_portrait(
    state: &AppState,
    user_id: i64,
    portrait_id: i64,
) -> Result<(), String> {
    portrait_project_id_for_user(state, user_id, portrait_id).await?;
    state
        .portrait
        .set_primary(portrait_id)
        .await
        .map_err(|e| e.to_string())
}

async fn portrait_project_id(state: &AppState, portrait_id: i64) -> Result<i64, String> {
    let portrait = character_portrait::Entity::find_by_id(portrait_id)
        .one(state.db.as_ref())
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "立绘不存在".to_string())?;
    Ok(portrait.project_id)
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
    let profile = protagonist_profile::Entity::find_by_id(profile_id)
        .one(state.db.as_ref())
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "角色档案不存在".to_string())?;
    Ok(profile.project_id)
}
