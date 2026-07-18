use serde_json::Value;

use crate::import_reverse::application::reverse_json::{optional_i32, required_i32};

pub(in crate::import_reverse::application::apply) fn required_positive_i32(
    value: &Value,
    key: &str,
    context: &str,
) -> Result<i32, String> {
    let number = required_i32(value, key, context)?;
    ensure_positive_i32(number, key, context)?;
    Ok(number)
}

pub(in crate::import_reverse::application::apply) fn optional_positive_i32(
    value: &Value,
    key: &str,
    context: &str,
) -> Result<Option<i32>, String> {
    let number = optional_i32(value, key, context)?;
    if let Some(number) = number {
        ensure_positive_i32(number, key, context)?;
    }
    Ok(number)
}

fn ensure_positive_i32(number: i32, key: &str, context: &str) -> Result<(), String> {
    if number > 0 {
        Ok(())
    } else {
        Err(format!("{context}.{key} 必须大于 0"))
    }
}
