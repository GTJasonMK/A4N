use crate::AppState;
use crate::actions::{access, app_error_message};
use a4n_core::models::character::protagonist_profile;
use a4n_orchestrator::project::required_text;

pub(super) async fn require_profile_for_user(
    state: &AppState,
    user_id: i64,
    profile_id: i64,
) -> Result<protagonist_profile::Model, String> {
    let profile = state
        .character()
        .profile_by_id(profile_id)
        .await
        .map_err(app_error_message)?
        .ok_or_else(|| format!("角色档案不存在: {profile_id}"))?;
    verify_profile_owner(state, user_id, &profile).await?;
    Ok(profile)
}

pub(super) async fn verify_profile_owner(
    state: &AppState,
    user_id: i64,
    profile: &protagonist_profile::Model,
) -> Result<(), String> {
    access::verify_project_editor_string(state, user_id, profile.project_id).await?;
    Ok(())
}

pub(super) async fn find_profile_by_name_for_user(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    name: &str,
) -> Result<protagonist_profile::Model, String> {
    access::verify_project_editor_string(state, user_id, project_id).await?;
    let name = required_text(name, "角色名称")?;
    state
        .character()
        .profile_by_project_name(project_id, &name)
        .await
        .map_err(app_error_message)?
        .ok_or_else(|| format!("角色不存在: project_id={project_id}, name={name}"))
}
