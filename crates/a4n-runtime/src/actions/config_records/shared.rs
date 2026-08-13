use crate::{
    AppState,
    actions::admin_records::{AdminPermission, require_admin_permission},
};
use a4n_core::error::AppError;

pub use crate::actions::request_preview::{ModelRequestPreview, ModelRequestPreviewStep};
pub use a4n_api_contract::config::ModelDiscoveryInput;
pub use a4n_model_discovery::{DiscoveredModel, ModelDiscoveryResult};

pub(super) async fn require_config_admin(
    state: &AppState,
    caller_user_id: i64,
) -> Result<(), AppError> {
    require_admin_permission(state, caller_user_id, AdminPermission::ManageSettings).await
}

pub(super) fn has_configured_api_key(value: Option<&str>) -> bool {
    value.is_some_and(|value| !value.trim().is_empty())
}

pub(super) fn validate_runtime_snapshot_config_id(
    config_kind: &str,
    snapshot_id: i64,
    config_id: i64,
) -> Result<(), String> {
    if snapshot_id == config_id {
        return Ok(());
    }
    Err(format!(
        "runtime_config_snapshot.{config_kind}.id 必须等于 config_id：snapshot_id={snapshot_id}, config_id={config_id}",
    ))
}
