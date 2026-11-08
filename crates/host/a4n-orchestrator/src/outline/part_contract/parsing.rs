use crate::outline::chapter_contract::parse_outline_json_object;
use a4n_error::AppError;
use serde_json::Value;

pub fn parse_part_outline_single_response(
    content: &str,
    part_number: i32,
    start_chapter: i32,
    end_chapter: i32,
) -> Result<Value, AppError> {
    let part_json = parse_outline_json_object(content, "part_outline_single")?;
    a4n_domain::json_contract::OutputSchemaValidator::new(
        &a4n_domain::chapter::part_outline_output_schema(),
    )
    .map_err(AppError::ConfigError)?
    .validate(&part_json)
    .map_err(AppError::InvalidArgument)?;
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
