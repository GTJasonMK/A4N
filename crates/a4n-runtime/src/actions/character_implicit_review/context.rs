use super::BehaviorForReview;
use crate::{AppState, actions::access};
use a4n_core::models::character::protagonist_profile;

pub(super) async fn implicit_review_context_for_user(
    state: &AppState,
    user_id: i64,
    profile_id: i64,
) -> Result<(protagonist_profile::Model, Vec<BehaviorForReview>), String> {
    let profile = state
        .orchestrator
        .profile_by_id(profile_id)
        .await
        .map_err(|err| err.to_string())?
        .ok_or_else(|| format!("角色档案不存在: {profile_id}"))?;
    access::verify_project_editor_string(state, user_id, profile.project_id).await?;
    let records = state
        .orchestrator
        .implicit_review_behavior_records(profile_id)
        .await
        .map_err(crate::actions::app_error_message)?;
    Ok((
        profile,
        records.iter().map(BehaviorForReview::from).collect(),
    ))
}

pub(super) async fn verify_profile_ownership(
    state: &AppState,
    user_id: i64,
    profile_id: i64,
) -> Result<(), String> {
    let profile = state
        .orchestrator
        .profile_by_id(profile_id)
        .await
        .map_err(|err| err.to_string())?
        .ok_or_else(|| format!("角色档案不存在: {profile_id}"))?;
    access::verify_project_editor_string(state, user_id, profile.project_id).await
}

pub(super) fn implicit_attribute_value(
    current_state: &serde_json::Value,
    attribute_key: &str,
) -> Result<serde_json::Value, String> {
    category_attributes(current_state, "implicit")
        .and_then(|attributes| attributes.get(attribute_key))
        .cloned()
        .ok_or_else(|| format!("隐性属性不存在: {attribute_key}"))
}

fn category_attributes<'a>(
    value: &'a serde_json::Value,
    category: &str,
) -> Option<&'a serde_json::Map<String, serde_json::Value>> {
    let key = match category {
        "explicit" => "explicit_attributes",
        "implicit" => "implicit_attributes",
        "social" => "social_attributes",
        other => other,
    };
    value.get(key)?.as_object()
}
