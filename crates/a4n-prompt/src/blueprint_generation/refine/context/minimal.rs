use a4n_core::error::AppError;
use serde_json::{Map, Value, json};

pub(super) fn minimal_blueprint_refine_context(
    blueprint_context: &Value,
) -> Result<Value, AppError> {
    let object = blueprint_context
        .as_object()
        .ok_or_else(|| AppError::InvalidState {
            current: "blueprint_refine_context_not_object".to_string(),
            operation: "压缩蓝图优化上下文".to_string(),
        })?;
    let mut minimal = serde_json::Map::new();
    let blueprint = required_object(object, "blueprint")?;
    minimal.insert(
        "blueprint".to_string(),
        minimal_blueprint_model_context(blueprint),
    );
    minimal.insert(
        "character_count".to_string(),
        json!(required_array_len(object, "characters")?),
    );
    minimal.insert(
        "relationship_count".to_string(),
        json!(required_array_len(object, "relationships")?),
    );
    Ok(Value::Object(minimal))
}

fn minimal_blueprint_model_context(object: &Map<String, Value>) -> Value {
    let mut minimal = serde_json::Map::new();
    for key in [
        "title",
        "genre",
        "style",
        "tone",
        "target_audience",
        "one_sentence_summary",
        "full_synopsis",
        "total_chapters",
        "chapters_per_part",
    ] {
        if let Some(value) = object.get(key) {
            minimal.insert(key.to_string(), value.clone());
        }
    }
    Value::Object(minimal)
}

fn required_object<'a>(
    context: &'a Map<String, Value>,
    key: &str,
) -> Result<&'a Map<String, Value>, AppError> {
    context
        .get(key)
        .and_then(Value::as_object)
        .ok_or_else(|| AppError::InvalidState {
            current: format!("blueprint_refine_context.{key}_not_object"),
            operation: "压缩蓝图优化上下文".to_string(),
        })
}

fn required_array_len(context: &Map<String, Value>, key: &str) -> Result<usize, AppError> {
    context
        .get(key)
        .and_then(Value::as_array)
        .map(Vec::len)
        .ok_or_else(|| AppError::InvalidState {
            current: format!("blueprint_refine_context.{key}_not_array"),
            operation: "压缩蓝图优化上下文".to_string(),
        })
}
