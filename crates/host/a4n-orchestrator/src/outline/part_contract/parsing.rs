use crate::outline::chapter_contract::{
    parse_outline_json_object, reject_keys, reject_unknown_keys, required_array, required_object,
    required_string,
};
use a4n_error::AppError;
use serde_json::Value;

pub fn parse_part_outline_single_response(
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
