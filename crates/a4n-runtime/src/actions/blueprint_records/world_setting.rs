use a4n_core::error::AppError;
use serde_json::Value;

use crate::AppState;
use crate::actions::access;

use super::contracts::UpdateWorldSettingInput;

pub async fn update_world_setting(
    state: &AppState,
    user_id: i64,
    input: UpdateWorldSettingInput,
) -> Result<Value, AppError> {
    access::verify_project_editor(state, user_id, input.project_id).await?;
    let world_setting = required_world_setting_value(&input.world_setting)?;
    state
        .orchestrator
        .update_blueprint_world_setting(input.project_id, world_setting.clone())
        .await?
        .ok_or_else(|| AppError::NotFound("蓝图不存在".to_string()))?;
    Ok(world_setting)
}

pub(super) fn required_world_setting_value(value: &Value) -> Result<Value, AppError> {
    if value.is_null() {
        return Err(AppError::InvalidArgument(
            "world_setting 不能为 null".to_string(),
        ));
    }
    Ok(value.clone())
}
