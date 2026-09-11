//! 角色属性差异。

use a4n_error::AppError;
use serde_json::{Map, Value, json};

pub(super) fn required_category_attributes<'a>(
    value: &'a Value,
    category: &str,
    side: &str,
) -> Result<&'a Map<String, Value>, AppError> {
    let direct_key = match category {
        "explicit" => "explicit_attributes",
        "implicit" => "implicit_attributes",
        "social" => "social_attributes",
        _ => category,
    };
    value
        .get("attributes")
        .and_then(Value::as_object)
        .ok_or_else(|| {
            AppError::InvalidArgument(format!("角色快照 {side}.attributes 缺少或不是对象"))
        })?
        .get(direct_key)
        .and_then(Value::as_object)
        .ok_or_else(|| {
            AppError::InvalidArgument(format!(
                "角色快照 {side}.attributes.{direct_key} 缺少或不是对象"
            ))
        })
}

pub(super) fn diff_attributes(from: &Map<String, Value>, to: &Map<String, Value>) -> Value {
    let mut added = Map::new();
    let mut modified = Map::new();
    let mut deleted = Map::new();

    for (key, value) in to {
        match from.get(key) {
            Some(old_value) if old_value != value => {
                modified.insert(key.clone(), json!({"from": old_value, "to": value}));
            }
            Some(_) => {}
            None => {
                added.insert(key.clone(), value.clone());
            }
        }
    }

    for (key, value) in from {
        if !to.contains_key(key) {
            deleted.insert(key.clone(), value.clone());
        }
    }

    json!({"added": added, "modified": modified, "deleted": deleted})
}
