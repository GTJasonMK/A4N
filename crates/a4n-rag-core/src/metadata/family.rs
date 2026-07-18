use a4n_core::error::AppError;

pub(super) fn validate_project_family_metadata(
    project_id: i64,
    object: &serde_json::Map<String, serde_json::Value>,
) -> Result<(), AppError> {
    let root_project_id = required_positive_i64(object, "root_project_id")?;
    let source_snapshot_id = required_nullable_positive_i64(object, "source_snapshot_id")?;
    let source_chapter_id = required_nullable_positive_i64(object, "source_chapter_id")?;
    let source_snapshot_chapter_id =
        required_nullable_positive_i64(object, "source_snapshot_chapter_id")?;
    let derived_project_id = required_nullable_positive_i64(object, "derived_project_id")?;
    match derived_project_id {
        Some(derived_project_id) => {
            if derived_project_id != project_id {
                return Err(AppError::InvalidArgument(format!(
                    "RAG metadata derived_project_id 必须等于当前项目: project_id={project_id}, derived_project_id={derived_project_id}"
                )));
            }
            if root_project_id == derived_project_id {
                return Err(AppError::InvalidArgument(format!(
                    "RAG metadata 派生项目不能同时作为文本根项目: project_id={project_id}"
                )));
            }
            if source_snapshot_id.is_none() {
                return Err(AppError::InvalidArgument(
                    "派生项目 RAG metadata 必须包含 source_snapshot_id".to_string(),
                ));
            }
        }
        None if root_project_id != project_id => {
            return Err(AppError::InvalidArgument(format!(
                "根文本 RAG metadata root_project_id 必须等于 project_id: project_id={project_id}, root_project_id={root_project_id}"
            )));
        }
        None => {}
    }
    if source_snapshot_chapter_id.is_some()
        && (source_snapshot_id.is_none() || source_chapter_id.is_none())
    {
        return Err(AppError::InvalidArgument("RAG metadata 设置 source_snapshot_chapter_id 时必须同时设置 source_snapshot_id 和 source_chapter_id".to_string()));
    }
    Ok(())
}

fn required_positive_i64(
    object: &serde_json::Map<String, serde_json::Value>,
    key: &str,
) -> Result<i64, AppError> {
    let value = object
        .get(key)
        .ok_or_else(|| AppError::InvalidArgument(format!("RAG metadata 缺少 {key}")))?;
    positive_i64(value)
        .ok_or_else(|| AppError::InvalidArgument(format!("RAG metadata {key} 必须为正整数")))
}

fn required_nullable_positive_i64(
    object: &serde_json::Map<String, serde_json::Value>,
    key: &str,
) -> Result<Option<i64>, AppError> {
    let value = object
        .get(key)
        .ok_or_else(|| AppError::InvalidArgument(format!("RAG metadata 缺少 {key}")))?;
    if value.is_null() {
        return Ok(None);
    }
    positive_i64(value)
        .map(Some)
        .ok_or_else(|| AppError::InvalidArgument(format!("RAG metadata {key} 必须为正整数或 null")))
}

fn positive_i64(value: &serde_json::Value) -> Option<i64> {
    let parsed = value
        .as_i64()
        .or_else(|| value.as_u64().and_then(|value| i64::try_from(value).ok()))?;
    (parsed > 0).then_some(parsed)
}
