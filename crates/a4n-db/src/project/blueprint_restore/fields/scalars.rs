use serde_json::Value;

use crate::project::blueprint_restore::BlueprintRestoreError;

const MIN_BLUEPRINT_TOTAL_CHAPTERS: i32 = 5;
const MAX_BLUEPRINT_TOTAL_CHAPTERS: i32 = 10_000;

pub(in crate::project::blueprint_restore) fn required_blueprint_snapshot_string(
    value: &Value,
    key: &str,
    label: &str,
) -> Result<String, BlueprintRestoreError> {
    optional_blueprint_snapshot_string(value, key, label)?.ok_or_else(|| {
        BlueprintRestoreError::invalid_snapshot(format!("{label} 缺少必填字符串字段: {key}"))
    })
}

pub(in crate::project::blueprint_restore) fn optional_blueprint_snapshot_string(
    value: &Value,
    key: &str,
    label: &str,
) -> Result<Option<String>, BlueprintRestoreError> {
    match value.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(text)) => {
            let text = text.trim();
            if text.is_empty() {
                Ok(None)
            } else {
                Ok(Some(text.to_string()))
            }
        }
        Some(_) => Err(BlueprintRestoreError::invalid_snapshot(format!(
            "{label}.{key} 必须是字符串"
        ))),
    }
}

pub(super) fn required_blueprint_snapshot_bool(
    value: &Value,
    key: &str,
    label: &str,
) -> Result<bool, BlueprintRestoreError> {
    value.get(key).and_then(Value::as_bool).ok_or_else(|| {
        BlueprintRestoreError::invalid_snapshot(format!("{label} 缺少必填布尔字段: {key}"))
    })
}

fn required_blueprint_snapshot_i32(
    value: &Value,
    key: &str,
    label: &str,
) -> Result<i32, BlueprintRestoreError> {
    optional_blueprint_snapshot_i32(value, key, label)?.ok_or_else(|| {
        BlueprintRestoreError::invalid_snapshot(format!("{label} 缺少必填整数字段: {key}"))
    })
}

pub(super) fn required_blueprint_snapshot_total_chapters(
    value: &Value,
    key: &str,
    label: &str,
) -> Result<i32, BlueprintRestoreError> {
    let total_chapters = required_blueprint_snapshot_i32(value, key, label)?;
    validate_blueprint_restore_chapter_count(total_chapters, &format!("{label}.{key}"))?;
    Ok(total_chapters)
}

fn optional_blueprint_snapshot_i32(
    value: &Value,
    key: &str,
    label: &str,
) -> Result<Option<i32>, BlueprintRestoreError> {
    match value.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(raw) => {
            let number = raw.as_i64().ok_or_else(|| {
                BlueprintRestoreError::invalid_snapshot(format!("{label}.{key} 必须是整数"))
            })?;
            i32::try_from(number).map(Some).map_err(|_| {
                BlueprintRestoreError::invalid_snapshot(format!(
                    "{label}.{key} 超出 i32 范围: {number}"
                ))
            })
        }
    }
}

pub(super) fn optional_positive_blueprint_snapshot_i32(
    value: &Value,
    key: &str,
    label: &str,
) -> Result<Option<i32>, BlueprintRestoreError> {
    let Some(number) = optional_blueprint_snapshot_i32(value, key, label)? else {
        return Ok(None);
    };
    if number <= 0 {
        return Err(BlueprintRestoreError::invalid_snapshot(format!(
            "{label}.{key} 必须为正整数"
        )));
    }
    Ok(Some(number))
}

pub(in crate::project::blueprint_restore) fn optional_blueprint_snapshot_strength(
    value: &Value,
    key: &str,
    label: &str,
) -> Result<Option<i32>, BlueprintRestoreError> {
    let Some(strength) = optional_blueprint_snapshot_i32(value, key, label)? else {
        return Ok(None);
    };
    if !(1..=10).contains(&strength) {
        return Err(BlueprintRestoreError::invalid_snapshot(format!(
            "{label}.{key} 必须在 1 到 10 之间"
        )));
    }
    Ok(Some(strength))
}

pub(in crate::project::blueprint_restore) fn optional_blueprint_snapshot_json(
    value: &Value,
    key: &str,
) -> Option<Value> {
    value.get(key).filter(|item| !item.is_null()).cloned()
}

fn validate_blueprint_restore_chapter_count(
    value: i32,
    context: &str,
) -> Result<(), BlueprintRestoreError> {
    if (MIN_BLUEPRINT_TOTAL_CHAPTERS..=MAX_BLUEPRINT_TOTAL_CHAPTERS).contains(&value) {
        Ok(())
    } else {
        Err(BlueprintRestoreError::invalid_snapshot(format!(
            "{context} 必须在 {MIN_BLUEPRINT_TOTAL_CHAPTERS} 到 {MAX_BLUEPRINT_TOTAL_CHAPTERS} 之间"
        )))
    }
}
