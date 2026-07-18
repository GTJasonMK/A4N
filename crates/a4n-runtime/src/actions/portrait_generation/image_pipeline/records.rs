use a4n_core::models::character::{character_portrait, protagonist_profile};

use crate::AppState;

use super::super::types::PortraitSummary;

pub(in crate::actions::portrait_generation) async fn character_portrait_by_id(
    state: &AppState,
    portrait_id: i64,
) -> Result<character_portrait::Model, String> {
    state
        .orchestrator
        .character_portrait_by_id(portrait_id)
        .await
        .map_err(|err| err.to_string())?
        .ok_or_else(|| "立绘不存在".to_string())
}

pub(in crate::actions::portrait_generation) async fn portrait_profile_by_id(
    state: &AppState,
    profile_id: i64,
    portrait_id: i64,
) -> Result<protagonist_profile::Model, String> {
    state
        .orchestrator
        .profile_by_id(profile_id)
        .await
        .map_err(|err| err.to_string())?
        .ok_or_else(|| format!("立绘 #{portrait_id} 缺少角色档案 #{profile_id}"))
}

pub(crate) fn to_summary(portrait: &character_portrait::Model) -> PortraitSummary {
    PortraitSummary {
        id: portrait.id,
        profile_id: portrait.profile_id,
        project_id: portrait.project_id,
        portrait_type: portrait.portrait_type.clone(),
        image_path: portrait.image_path.clone(),
        prompt: portrait.prompt.clone(),
        expression: portrait.expression.clone(),
        pose: portrait.pose.clone(),
        is_primary: portrait.is_primary,
        created_at: Some(portrait.created_at.to_string()),
    }
}
