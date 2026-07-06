use a4n_core::error::AppError;
use a4n_core::models::chapter::chapter;
use chrono::Utc;
use serde_json::{Map, Value, json};
use sha2::{Digest, Sha256};

pub(super) fn normalize_analysis(raw: Value, ch: &chapter::Model) -> Result<Value, AppError> {
    let mut root = raw
        .as_object()
        .cloned()
        .ok_or_else(|| analysis_schema_error("chapter_analysis 必须返回 JSON 对象"))?;
    validate_analysis_schema(&root)?;

    root.insert(
        "_meta".to_string(),
        json!({
            "project_id": ch.project_id,
            "chapter_id": ch.id,
            "chapter_number": ch.chapter_number,
            "content_sha256": content_sha256(&ch.content),
            "generated_at": Utc::now().naive_utc().to_string(),
        }),
    );

    Ok(Value::Object(root))
}

pub(super) fn validate_saved_analysis(value: &Value) -> Result<(), AppError> {
    let mut root = value
        .as_object()
        .cloned()
        .ok_or_else(|| analysis_schema_error("chapter_analysis 必须是 JSON 对象"))?;
    root.remove("_meta");
    validate_analysis_schema(&root)
}

fn validate_analysis_schema(root: &Map<String, Value>) -> Result<(), AppError> {
    reject_unknown_fields(
        root,
        &[
            "metadata",
            "summaries",
            "character_states",
            "foreshadowing",
            "key_events",
        ],
        "chapter_analysis",
    )?;

    let metadata = required_object(root, "metadata", "chapter_analysis")?;
    reject_unknown_fields(
        metadata,
        &[
            "characters",
            "locations",
            "items",
            "tags",
            "tone",
            "timeline_marker",
        ],
        "chapter_analysis.metadata",
    )?;
    for key in ["characters", "locations", "items", "tags"] {
        required_array(metadata, key, "chapter_analysis.metadata")?;
    }
    required_string_or_null(metadata, "tone", "chapter_analysis.metadata")?;
    required_string_or_null(metadata, "timeline_marker", "chapter_analysis.metadata")?;

    let summaries = required_object(root, "summaries", "chapter_analysis")?;
    reject_unknown_fields(
        summaries,
        &["compressed", "one_line", "keywords"],
        "chapter_analysis.summaries",
    )?;
    required_string(summaries, "compressed", "chapter_analysis.summaries")?;
    required_string(summaries, "one_line", "chapter_analysis.summaries")?;
    required_array(summaries, "keywords", "chapter_analysis.summaries")?;

    let character_states = required_object(root, "character_states", "chapter_analysis")?;
    for (name, state) in character_states {
        if name.trim().is_empty() {
            return Err(analysis_schema_error(
                "chapter_analysis.character_states 角色名不能为空",
            ));
        }
        let state = state.as_object().ok_or_else(|| {
            analysis_schema_error(format!(
                "chapter_analysis.character_states.{name} 必须是对象"
            ))
        })?;
        reject_unknown_fields(
            state,
            &["location", "status", "changes", "emotional_state"],
            &format!("chapter_analysis.character_states.{name}"),
        )?;
        required_string_or_null(state, "location", "chapter_analysis.character_states")?;
        required_string_or_null(state, "status", "chapter_analysis.character_states")?;
        required_array(state, "changes", "chapter_analysis.character_states")?;
        required_string_or_null(
            state,
            "emotional_state",
            "chapter_analysis.character_states",
        )?;
    }

    let foreshadowing = required_object(root, "foreshadowing", "chapter_analysis")?;
    reject_unknown_fields(
        foreshadowing,
        &["planted", "resolved", "tensions"],
        "chapter_analysis.foreshadowing",
    )?;
    let planted = required_array(foreshadowing, "planted", "chapter_analysis.foreshadowing")?;
    for (index, item) in planted.iter().enumerate() {
        let context = format!("chapter_analysis.foreshadowing.planted[{index}]");
        let item = item
            .as_object()
            .ok_or_else(|| analysis_schema_error(format!("{context} 必须是对象")))?;
        reject_unknown_fields(
            item,
            &[
                "description",
                "original_text",
                "category",
                "priority",
                "related_entities",
            ],
            &context,
        )?;
        required_string(item, "description", &context)?;
        required_string(item, "original_text", &context)?;
        required_string(item, "category", &context)?;
        required_string(item, "priority", &context)?;
        required_array(item, "related_entities", &context)?;
    }
    let resolved = required_array(foreshadowing, "resolved", "chapter_analysis.foreshadowing")?;
    for (index, item) in resolved.iter().enumerate() {
        let context = format!("chapter_analysis.foreshadowing.resolved[{index}]");
        let item = item
            .as_object()
            .ok_or_else(|| analysis_schema_error(format!("{context} 必须是对象")))?;
        reject_unknown_fields(item, &["id", "resolution"], &context)?;
        required_string(item, "id", &context)?;
        required_string(item, "resolution", &context)?;
    }
    let tensions = required_array(foreshadowing, "tensions", "chapter_analysis.foreshadowing")?;
    for (index, item) in tensions.iter().enumerate() {
        if item.as_str().map(str::trim).is_none_or(str::is_empty) {
            return Err(analysis_schema_error(format!(
                "chapter_analysis.foreshadowing.tensions[{index}] 必须是非空字符串"
            )));
        }
    }
    let key_events = required_array(root, "key_events", "chapter_analysis")?;
    for (index, item) in key_events.iter().enumerate() {
        let context = format!("chapter_analysis.key_events[{index}]");
        let item = item
            .as_object()
            .ok_or_else(|| analysis_schema_error(format!("{context} 必须是对象")))?;
        reject_unknown_fields(
            item,
            &["type", "description", "importance", "involved_characters"],
            &context,
        )?;
        required_string(item, "type", &context)?;
        required_string(item, "description", &context)?;
        required_string(item, "importance", &context)?;
        required_array(item, "involved_characters", &context)?;
    }
    Ok(())
}

fn reject_unknown_fields(
    object: &Map<String, Value>,
    allowed: &[&str],
    context: &str,
) -> Result<(), AppError> {
    for key in object.keys() {
        if !allowed.contains(&key.as_str()) {
            return Err(analysis_schema_error(format!(
                "{context} 包含未知字段 {key}；允许字段：{}",
                allowed.join(", ")
            )));
        }
    }
    Ok(())
}

fn required_object<'a>(
    root: &'a Map<String, Value>,
    key: &str,
    context: &str,
) -> Result<&'a Map<String, Value>, AppError> {
    root.get(key)
        .ok_or_else(|| analysis_schema_error(format!("{context} 缺少必需字段 {key}")))?
        .as_object()
        .ok_or_else(|| analysis_schema_error(format!("{context}.{key} 必须是对象")))
}

fn required_array<'a>(
    root: &'a Map<String, Value>,
    key: &str,
    context: &str,
) -> Result<&'a Vec<Value>, AppError> {
    root.get(key)
        .ok_or_else(|| analysis_schema_error(format!("{context} 缺少必需字段 {key}")))?
        .as_array()
        .ok_or_else(|| analysis_schema_error(format!("{context}.{key} 必须是数组")))
}

fn required_string<'a>(
    root: &'a Map<String, Value>,
    key: &str,
    context: &str,
) -> Result<&'a str, AppError> {
    root.get(key)
        .ok_or_else(|| analysis_schema_error(format!("{context} 缺少必需字段 {key}")))?
        .as_str()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| analysis_schema_error(format!("{context}.{key} 必须是非空字符串")))
}

fn required_string_or_null(
    root: &Map<String, Value>,
    key: &str,
    context: &str,
) -> Result<(), AppError> {
    match root.get(key) {
        Some(Value::String(value)) if !value.trim().is_empty() => Ok(()),
        Some(Value::String(_)) => Err(analysis_schema_error(format!(
            "{context}.{key} 必须是非空字符串或 null"
        ))),
        Some(Value::Null) => Ok(()),
        Some(_) => Err(analysis_schema_error(format!(
            "{context}.{key} 必须是字符串或 null"
        ))),
        None => Err(analysis_schema_error(format!(
            "{context} 缺少必需字段 {key}"
        ))),
    }
}

pub(super) fn analysis_schema_error(message: impl Into<String>) -> AppError {
    AppError::InvalidArgument(message.into())
}

pub(super) fn content_sha256(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    let bytes = hasher.finalize();
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
