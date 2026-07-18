use crate::AppState;
use serde_json::Value;

pub(super) async fn resolve_reference_bound_asset(
    state: &AppState,
    project_id: i64,
    asset_kind: &str,
    asset_id: i64,
) -> Result<Value, String> {
    state
        .orchestrator
        .manga_reference_bound_asset(project_id, asset_kind, asset_id)
        .await
        .map_err(crate::actions::app_error_message)
}
