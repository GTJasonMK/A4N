use a4n_core::error::AppError;
use a4n_core::models::character::protagonist_profile;
use sea_orm::Set;

fn category_storage_key(category: &str) -> &'static str {
    match category {
        "explicit" => "explicit_attributes",
        "implicit" => "implicit_attributes",
        "social" => "social_attributes",
        _ => "attributes",
    }
}

pub(super) fn apply_attribute_to_state(
    state: &mut serde_json::Value,
    category: &str,
    key: &str,
    operation: &str,
    new_value: serde_json::Value,
) -> Result<(), AppError> {
    let storage_key = category_storage_key(category);
    let root = state
        .as_object_mut()
        .ok_or_else(|| AppError::InvalidArgument("角色当前状态必须是 JSON 对象".to_string()))?;
    let attributes = root
        .get_mut(storage_key)
        .ok_or_else(|| AppError::InvalidArgument(format!("角色当前状态缺少 {storage_key}")))?
        .as_object_mut()
        .ok_or_else(|| {
            AppError::InvalidArgument(format!("角色当前状态 {storage_key} 必须是 JSON 对象"))
        })?;
    if operation == "delete" {
        attributes.remove(key);
    } else {
        attributes.insert(key.to_string(), new_value);
    }
    Ok(())
}

pub(super) fn attribute_value_in_state(
    state: &serde_json::Value,
    category: &str,
    key: &str,
) -> Option<serde_json::Value> {
    state
        .get(category_storage_key(category))
        .and_then(serde_json::Value::as_object)
        .and_then(|attributes| attributes.get(key))
        .cloned()
}

pub fn snapshot_attributes_from_data(
    data: &serde_json::Value,
    chapter_number: i32,
) -> Result<serde_json::Value, AppError> {
    let attributes = data.get("attributes").ok_or_else(|| {
        AppError::InvalidArgument(format!("第 {chapter_number} 章角色快照缺少 attributes"))
    })?;
    let object = attributes.as_object().ok_or_else(|| {
        AppError::InvalidArgument(format!(
            "第 {chapter_number} 章角色快照 attributes 不是 JSON 对象"
        ))
    })?;

    for key in [
        "explicit_attributes",
        "implicit_attributes",
        "social_attributes",
    ] {
        if !object.get(key).is_some_and(serde_json::Value::is_object) {
            return Err(AppError::InvalidArgument(format!(
                "第 {chapter_number} 章角色快照 attributes.{key} 缺少或不是 JSON 对象"
            )));
        }
    }

    Ok(attributes.clone())
}

fn optional_profile_text_field(
    attributes: &serde_json::Value,
    category: &str,
    key: &str,
) -> Option<String> {
    attributes
        .get(category_storage_key(category))
        .and_then(serde_json::Value::as_object)
        .and_then(|values| values.get(key))
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .map(ToOwned::to_owned)
}

pub fn apply_profile_current_state(
    active: &mut protagonist_profile::ActiveModel,
    current_state: serde_json::Value,
) {
    active.gender = Set(optional_profile_text_field(
        &current_state,
        "explicit",
        "gender",
    ));
    active.age = Set(optional_profile_text_field(
        &current_state,
        "explicit",
        "age",
    ));
    active.appearance = Set(optional_profile_text_field(
        &current_state,
        "explicit",
        "appearance",
    ));
    active.background = Set(optional_profile_text_field(
        &current_state,
        "explicit",
        "background",
    ));
    active.personality = Set(optional_profile_text_field(
        &current_state,
        "implicit",
        "personality",
    ));
    active.current_state = Set(current_state);
}

pub(super) fn normalize_base_attributes(
    base: serde_json::Value,
) -> Result<serde_json::Value, AppError> {
    let Some(root) = base.as_object() else {
        return Err(AppError::InvalidArgument(
            "初始角色属性必须是 JSON 对象".to_string(),
        ));
    };

    for key in [
        "explicit_attributes",
        "implicit_attributes",
        "social_attributes",
    ] {
        if !root.get(key).is_some_and(serde_json::Value::is_object) {
            return Err(AppError::InvalidArgument(format!(
                "初始角色属性 {key} 缺少或不是 JSON 对象"
            )));
        }
    }

    for key in root.keys() {
        if !matches!(
            key.as_str(),
            "explicit_attributes" | "implicit_attributes" | "social_attributes"
        ) {
            return Err(AppError::InvalidArgument(format!(
                "初始角色属性包含未知顶层字段: {key}"
            )));
        }
    }
    Ok(base)
}
