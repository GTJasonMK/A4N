use super::super::GalgameMediaBindingResult;
use super::rules::{
    normalize_galgame_media_role, normalize_galgame_media_target_kind, validate_galgame_media_role,
};
use super::{BindGalgameMediaInput, UnbindGalgameMediaInput};
use crate::{AppState, actions::access};
#[cfg(test)]
use a4n_core::models::media::media_asset;
use a4n_orchestrator::galgame::GalgameTargetContext;

pub async fn bind_galgame_media(
    state: &AppState,
    user_id: i64,
    input: BindGalgameMediaInput,
) -> Result<GalgameMediaBindingResult, String> {
    access::verify_project_editor_string(state, user_id, input.project_id).await?;
    bind_galgame_media_inner(state, input).await
}

pub async fn unbind_galgame_media(
    state: &AppState,
    user_id: i64,
    input: UnbindGalgameMediaInput,
) -> Result<u64, String> {
    access::verify_project_editor_string(state, user_id, input.project_id).await?;
    let target_kind = normalize_galgame_media_target_kind(&input.target_kind)?;
    let role = normalize_galgame_media_role(&target_kind, &input.role)?;
    state
        .orchestrator
        .unbind_galgame_media_records(
            input.project_id,
            &target_kind,
            input.target_id,
            &role,
            input.media_asset_id,
        )
        .await
        .map_err(crate::actions::app_error_message)
}

pub(crate) async fn bind_galgame_media_inner(
    state: &AppState,
    input: BindGalgameMediaInput,
) -> Result<GalgameMediaBindingResult, String> {
    if input.media_asset_id <= 0 {
        return Err("media_asset_id 必须是正整数".to_string());
    }
    let target_kind = normalize_galgame_media_target_kind(&input.target_kind)?;
    let role = normalize_galgame_media_role(&target_kind, &input.role)?;
    let role_rule = validate_galgame_media_role(&target_kind, &role)?;
    state
        .orchestrator
        .bind_galgame_media_records(
            input.project_id,
            input.media_asset_id,
            &target_kind,
            input.target_id,
            &role,
            role_rule.required_media_kind,
        )
        .await
        .map_err(crate::actions::app_error_message)
}

pub(crate) async fn validate_galgame_media_target(
    state: &AppState,
    project_id: i64,
    target_kind: &str,
    target_id: i64,
) -> Result<GalgameTargetContext, String> {
    state
        .orchestrator
        .galgame_media_target_context(project_id, target_kind, target_id)
        .await
        .map_err(crate::actions::app_error_message)
}

#[cfg(test)]
pub(super) fn validate_galgame_media_asset_source_context(
    asset: &media_asset::Model,
    context: &GalgameTargetContext,
) -> Result<(), String> {
    a4n_orchestrator::galgame::validate_galgame_media_asset_source_context(asset, context)
        .map_err(|error| error.to_string())
}
