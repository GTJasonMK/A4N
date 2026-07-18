use crate::outline::chapter_contract::{
    parse_outline_json_object, reject_keys, reject_unknown_keys, required_array, required_object,
    required_string,
};
use a4n_core::error::AppError;
use serde_json::{Value, json};

pub(in crate::outline) fn part_outline_single_response_schema() -> Value {
    json!({
        "type": "object",
        "additionalProperties": false,
        "required": ["title", "summary", "theme", "key_events", "character_focus"],
        "properties": {
            "title": {"type": "string"},
            "summary": {"type": "string"},
            "theme": {"type": "string"},
            "key_events": {"type": "array", "items": {}},
            "character_focus": {"type": "object"}
        }
    })
}

pub(in crate::outline) fn parse_part_outline_single_response(
    content: &str,
    part_number: i32,
    start_chapter: i32,
    end_chapter: i32,
) -> Result<Value, AppError> {
    let part_json = parse_outline_json_object(content, "part_outline_single")?;
    reject_keys(
        &part_json,
        &["parts", "character_arcs"],
        "part_outline_single",
    )?;
    reject_unknown_keys(
        &part_json,
        &["title", "summary", "theme", "key_events", "character_focus"],
        "part_outline_single",
    )?;
    required_string(&part_json, "title")?;
    required_string(&part_json, "summary")?;
    required_string(&part_json, "theme")?;
    let key_events = required_array(&part_json, "key_events")?;
    let key_events = key_events
        .as_array()
        .expect("required_array guarantees JSON array");
    if key_events.len() < 3 {
        return Err(AppError::InvalidArgument(
            "part_outline_single.key_events 至少需要 3 个关键事件".to_string(),
        ));
    }
    required_object(&part_json, "character_focus")?;
    let mut materialized = part_json.as_object().cloned().ok_or_else(|| {
        AppError::InvalidArgument("part_outline_single 必须是 JSON 对象".to_string())
    })?;
    materialized.insert("part_number".to_string(), Value::Number(part_number.into()));
    materialized.insert(
        "start_chapter".to_string(),
        Value::Number(start_chapter.into()),
    );
    materialized.insert("end_chapter".to_string(), Value::Number(end_chapter.into()));
    Ok(Value::Object(materialized))
}

pub(in crate::outline) fn validate_materialized_part_outline(
    value: &Value,
    part_number: i32,
    start_chapter: i32,
    end_chapter: i32,
) -> Result<Value, AppError> {
    let mut raw = value.as_object().cloned().ok_or_else(|| {
        AppError::InvalidArgument("部大纲断点 parsed_output 必须是 JSON 对象".to_string())
    })?;
    for (key, expected) in [
        ("part_number", part_number),
        ("start_chapter", start_chapter),
        ("end_chapter", end_chapter),
    ] {
        let actual = raw
            .remove(key)
            .and_then(|value| value.as_i64())
            .ok_or_else(|| {
                AppError::InvalidArgument(format!("部大纲断点 parsed_output 缺少整数 {key}"))
            })?;
        if actual != i64::from(expected) {
            return Err(AppError::InvalidArgument(format!(
                "部大纲断点 parsed_output.{key} 必须等于 {expected}"
            )));
        }
    }
    parse_part_outline_single_response(
        &serde_json::to_string(&Value::Object(raw))?,
        part_number,
        start_chapter,
        end_chapter,
    )
}
