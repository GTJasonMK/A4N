use serde_json::Value;

use super::contracts::RagSyncError;

pub fn root_text_family_metadata(project_id: i64, metadata: Value) -> Result<Value, RagSyncError> {
    let Value::Object(mut object) = metadata else {
        return Err(RagSyncError::InvalidRecord {
            message: "RAG 记录 metadata 必须是 JSON 对象".to_string(),
        });
    };
    let source_chapter_id = optional_metadata_positive_i64(&object, "chapter_id")?
        .map(Value::from)
        .unwrap_or(Value::Null);
    insert_or_validate(
        &mut object,
        "root_project_id",
        serde_json::json!(project_id),
    )?;
    insert_or_validate(&mut object, "source_snapshot_id", Value::Null)?;
    insert_or_validate(&mut object, "source_chapter_id", source_chapter_id)?;
    insert_or_validate(&mut object, "source_snapshot_chapter_id", Value::Null)?;
    insert_or_validate(&mut object, "derived_project_id", Value::Null)?;
    Ok(Value::Object(object))
}

fn insert_or_validate(
    object: &mut serde_json::Map<String, Value>,
    key: &str,
    expected: Value,
) -> Result<(), RagSyncError> {
    match object.get(key) {
        Some(value) if value == &expected => Ok(()),
        Some(value) => Err(RagSyncError::InvalidRecord {
            message: format!(
                "RAG 记录 metadata {key} 与文本根项目上下文不一致: expected={expected}, actual={value}"
            ),
        }),
        None => {
            object.insert(key.to_string(), expected);
            Ok(())
        }
    }
}

fn optional_metadata_positive_i64(
    object: &serde_json::Map<String, Value>,
    key: &str,
) -> Result<Option<i64>, RagSyncError> {
    let Some(value) = object.get(key) else {
        return Ok(None);
    };
    let parsed = value
        .as_i64()
        .or_else(|| value.as_u64().and_then(|value| i64::try_from(value).ok()))
        .filter(|value| *value > 0)
        .ok_or_else(|| RagSyncError::InvalidRecord {
            message: format!("RAG 记录 metadata {key} 必须是正整数，实际为 {value}"),
        })?;
    Ok(Some(parsed))
}
