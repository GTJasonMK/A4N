use crate::{AppState, actions::app_error_message};
use a4n_core::models::character::protagonist_profile;
use a4n_orchestrator::text_anchor::EVIDENCE_ROLE_FIRST_SEEN;
use a4n_orchestrator::text_anchor::{TextAnchorEvidenceInput, TextAnchorProfileKind};
use serde_json::Value;

use super::super::json_utils::required_value;

pub(super) async fn bind_character_profile_evidence(
    state: &AppState,
    project_id: i64,
    profile_id: i64,
    character: &Value,
) -> Result<(), String> {
    let text_anchor_id = required_value(character, "evidence_anchor_id")?
        .as_i64()
        .ok_or_else(|| "角色发现字段 evidence_anchor_id 必须是 JSON 整数数字".to_string())?;
    state
        .orchestrator
        .merge_profile_evidence_records(
            project_id,
            None,
            TextAnchorProfileKind::Character,
            profile_id,
            vec![TextAnchorEvidenceInput {
                text_anchor_id,
                evidence_role: EVIDENCE_ROLE_FIRST_SEEN.to_string(),
            }],
        )
        .await
        .map_err(app_error_message)?;
    Ok(())
}

pub(super) async fn existing_character_profile_by_name(
    state: &AppState,
    project_id: i64,
    name: &str,
) -> Result<protagonist_profile::Model, String> {
    state
        .orchestrator
        .protagonist_profiles_for_project(project_id)
        .await
        .map_err(crate::actions::app_error_message)?
        .into_iter()
        .find(|profile| profile.name == name)
        .ok_or_else(|| {
            format!("角色档案已存在但无法重新读取: project_id={project_id}, name={name}")
        })
}
