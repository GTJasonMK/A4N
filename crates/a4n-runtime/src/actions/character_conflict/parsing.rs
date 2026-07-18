use a4n_core::error::AppError;
use serde_json::Value;
use std::collections::BTreeSet;

pub(super) fn parse_conflict_output_with_source_ids(
    content: &str,
    allowed_source_ids: &BTreeSet<String>,
) -> Result<Vec<Value>, AppError> {
    let conflicts = parse_conflict_output(content)?;
    for (index, conflict) in conflicts.iter().enumerate() {
        validate_conflict_source_id(index, conflict, "expected_source_id", allowed_source_ids)?;
        validate_conflict_source_id(index, conflict, "actual_source_id", allowed_source_ids)?;
    }
    Ok(conflicts)
}

pub(super) fn restored_character_conflict_checkpoint(
    checkpoint_json: Value,
    call_key: &str,
    allowed_source_ids: &BTreeSet<String>,
) -> Result<Vec<Value>, AppError> {
    let parsed_output = checkpoint_json
        .get("parsed_output")
        .and_then(Value::as_array)
        .cloned()
        .ok_or_else(|| AppError::InvalidState {
            current: "missing_or_invalid_checkpoint_field:parsed_output".to_string(),
            operation: format!("恢复角色冲突检查 LLM 调用断点 {call_key}"),
        })?;
    let response_content = checkpoint_json
        .get("response_content")
        .and_then(Value::as_str)
        .ok_or_else(|| AppError::InvalidState {
            current: "missing_or_invalid_checkpoint_field:response_content".to_string(),
            operation: format!("恢复角色冲突检查 LLM 调用断点 {call_key}"),
        })?;
    let reparsed = parse_conflict_output_with_source_ids(response_content, allowed_source_ids)?;
    if reparsed != parsed_output {
        return Err(AppError::InvalidState {
            current: "checkpoint_parsed_output_mismatch".to_string(),
            operation: format!("恢复角色冲突检查 LLM 调用断点 {call_key}"),
        });
    }
    Ok(reparsed)
}

pub(super) fn parse_conflict_output(content: &str) -> Result<Vec<Value>, AppError> {
    let value: Value = serde_json::from_str(content.trim()).map_err(|err| {
        AppError::InvalidArgument(format!("角色冲突检查结果必须是 JSON 数组: {err}"))
    })?;
    let conflicts = value
        .as_array()
        .ok_or_else(|| AppError::InvalidArgument("角色冲突检查结果必须是 JSON 数组".to_string()))?;
    for (index, conflict) in conflicts.iter().enumerate() {
        validate_conflict_item(index, conflict)?;
    }
    Ok(conflicts.clone())
}

fn validate_conflict_item(index: usize, conflict: &Value) -> Result<(), AppError> {
    let object = conflict
        .as_object()
        .ok_or_else(|| AppError::InvalidArgument(format!("conflicts[{index}] 必须是 JSON 对象")))?;
    reject_unknown_conflict_fields(index, object)?;
    required_i64(conflict, "chapter_number")?;
    required_string(conflict, "field")?;
    required_present(conflict, "expected")?;
    required_present(conflict, "actual")?;
    required_string(conflict, "expected_source_id")?;
    required_string(conflict, "actual_source_id")?;
    let severity = required_string(conflict, "severity")?;
    if !matches!(severity.as_str(), "minor" | "moderate" | "major") {
        return Err(AppError::InvalidArgument(format!(
            "conflicts[{index}].severity 必须是 minor/moderate/major"
        )));
    }
    Ok(())
}

fn reject_unknown_conflict_fields(
    index: usize,
    object: &serde_json::Map<String, Value>,
) -> Result<(), AppError> {
    const ALLOWED: &[&str] = &[
        "chapter_number",
        "field",
        "expected",
        "actual",
        "expected_source_id",
        "actual_source_id",
        "severity",
    ];
    for key in object.keys() {
        if !ALLOWED.contains(&key.as_str()) {
            return Err(AppError::InvalidArgument(format!(
                "conflicts[{index}] 包含未支持字段 `{key}`"
            )));
        }
    }
    Ok(())
}

fn validate_conflict_source_id(
    index: usize,
    conflict: &Value,
    field: &str,
    allowed_source_ids: &BTreeSet<String>,
) -> Result<(), AppError> {
    let source_id = required_string(conflict, field)?;
    if allowed_source_ids.contains(&source_id) {
        Ok(())
    } else {
        Err(AppError::InvalidArgument(format!(
            "conflicts[{index}].{field} 必须来自本次输入 source_id: {source_id}"
        )))
    }
}

fn required_i64(value: &Value, field: &str) -> Result<i64, AppError> {
    value
        .get(field)
        .and_then(Value::as_i64)
        .ok_or_else(|| AppError::InvalidArgument(format!("{field} 缺失或不是整数")))
}

fn required_string(value: &Value, field: &str) -> Result<String, AppError> {
    value
        .get(field)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .ok_or_else(|| AppError::InvalidArgument(format!("{field} 缺失或不是非空字符串")))
}

fn required_present<'a>(value: &'a Value, field: &str) -> Result<&'a Value, AppError> {
    value
        .get(field)
        .ok_or_else(|| AppError::InvalidArgument(format!("{field} 缺失")))
}
