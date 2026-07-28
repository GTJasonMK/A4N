use a4n_core::error::AppError;
use serde_json::Value;

use crate::AppState;

use super::contracts::UpdateWorldSettingInput;

pub async fn update_world_setting(
    state: &AppState,
    user_id: i64,
    input: UpdateWorldSettingInput,
) -> Result<Value, AppError> {
    state
        .project_blueprint()
        .update_blueprint_world_setting_for_user_validated(
            user_id,
            input.project_id,
            input.world_setting,
        )
        .await
}
