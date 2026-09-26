use a4n_db_models::models::character::protagonist_profile;
use sea_orm::Set;
use serde_json::Value;

use crate::character::CharacterDbError;
pub use a4n_domain::character::profile_current_state_fields;

pub(crate) fn apply_profile_current_state(
    active: &mut protagonist_profile::ActiveModel,
    current_state: Value,
) {
    let fields = profile_current_state_fields(current_state);
    active.gender = Set(fields.gender);
    active.age = Set(fields.age);
    active.appearance = Set(fields.appearance);
    active.background = Set(fields.background);
    active.personality = Set(fields.personality);
    active.current_state = Set(fields.current_state);
}

pub(super) fn normalize_base_attributes(base: Value) -> Result<Value, CharacterDbError> {
    let Some(root) = base.as_object() else {
        return Err(CharacterDbError::InvalidArgument(
            "初始角色属性必须是 JSON 对象".to_string(),
        ));
    };

    for key in [
        "explicit_attributes",
        "implicit_attributes",
        "social_attributes",
    ] {
        if !root.get(key).is_some_and(Value::is_object) {
            return Err(CharacterDbError::InvalidArgument(format!(
                "初始角色属性 {key} 缺少或不是 JSON 对象"
            )));
        }
    }

    for key in root.keys() {
        if !matches!(
            key.as_str(),
            "explicit_attributes" | "implicit_attributes" | "social_attributes"
        ) {
            return Err(CharacterDbError::InvalidArgument(format!(
                "初始角色属性包含未知顶层字段: {key}"
            )));
        }
    }
    Ok(base)
}
