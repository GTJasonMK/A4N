use super::contracts::{
    SCENE_ENTITY_STATE_KEYS, SCENE_ENTITY_TYPES, SCENE_TRANSITION_MOVEMENT_TYPES, SCENE_TYPES,
};
use a4n_domain::profile_text::required_text;
use a4n_error::AppError;

pub fn normalize_scene_type_value(value: &str) -> Result<String, AppError> {
    let value = required_text(value, "场景类型")?;
    if SCENE_TYPES.contains(&value.as_str()) {
        return Ok(value);
    }
    Err(AppError::InvalidArgument(format!(
        "场景类型无效: {value}；允许值：{}",
        SCENE_TYPES.join("/")
    )))
}

pub(super) fn validate_positive_i32(value: i32, field: &str) -> Result<i32, AppError> {
    if value <= 0 {
        Err(AppError::InvalidArgument(format!("{field} 必须大于 0")))
    } else {
        Ok(value)
    }
}

pub(super) fn trimmed_optional(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

pub fn normalize_movement_type_value(value: &str) -> Result<String, AppError> {
    let value = required_text(value, "位移类型")?;
    if SCENE_TRANSITION_MOVEMENT_TYPES.contains(&value.as_str()) {
        return Ok(value);
    }
    Err(AppError::InvalidArgument(format!(
        "位移类型无效: {value}；允许值：{}",
        SCENE_TRANSITION_MOVEMENT_TYPES.join("/")
    )))
}

pub fn normalize_scene_entity_type_value(value: &str) -> Result<String, AppError> {
    let value = required_text(value, "实体类型")?;
    if SCENE_ENTITY_TYPES.contains(&value.as_str()) {
        return Ok(value);
    }
    Err(AppError::InvalidArgument(format!(
        "实体类型无效: {value}；允许值：{}",
        SCENE_ENTITY_TYPES.join("/")
    )))
}

pub fn normalize_scene_entity_state_key_value(value: &str) -> Result<String, AppError> {
    let value = required_text(value, "实体状态键")?;
    if SCENE_ENTITY_STATE_KEYS.contains(&value.as_str()) {
        return Ok(value);
    }
    Err(AppError::InvalidArgument(format!(
        "实体状态键无效: {value}；允许值：{}",
        SCENE_ENTITY_STATE_KEYS.join("/")
    )))
}
