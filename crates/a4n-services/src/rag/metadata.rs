use a4n_core::error::AppError;
use a4n_core::rag::RAG_TEXT_INDEX_VERSION;
use a4n_engine::embedding::provider::EmbeddingModelIdentity;
use sha2::{Digest, Sha256};

use super::RagService;

impl RagService {
    pub fn content_hash(text: &str) -> Result<String, AppError> {
        let text = Self::required_content(text)?;
        Ok(content_sha256(&text))
    }

    pub fn metadata_content_hash(metadata: &serde_json::Value) -> Option<&str> {
        metadata
            .get("content_hash")
            .and_then(|value| value.as_str())
    }

    pub fn metadata_matches_current_text_index(metadata: &serde_json::Value) -> bool {
        metadata
            .get("text_index_version")
            .and_then(|value| value.as_str())
            == Some(RAG_TEXT_INDEX_VERSION)
    }

    pub fn metadata_matches_embedding_identity(
        metadata: &serde_json::Value,
        identity: &EmbeddingModelIdentity,
    ) -> bool {
        let dimension = metadata
            .get("embedding_dimension")
            .and_then(|value| value.as_u64())
            .and_then(|value| usize::try_from(value).ok());
        metadata
            .get("embedding_backend")
            .and_then(|value| value.as_str())
            == Some(identity.backend.as_str())
            && metadata
                .get("embedding_model")
                .and_then(|value| value.as_str())
                == Some(identity.model.as_str())
            && dimension == Some(identity.dimension)
    }

    pub(super) fn normalize_metadata(
        project_id: i64,
        text: &str,
        metadata: serde_json::Value,
        embedding_identity: &EmbeddingModelIdentity,
    ) -> Result<serde_json::Value, AppError> {
        let content_hash = content_sha256(text);
        let content_length = text.chars().count();
        match metadata {
            serde_json::Value::Object(mut object) => {
                validate_project_family_metadata(project_id, &object)?;
                object.insert("project_id".to_string(), serde_json::json!(project_id));
                object.insert("content_hash".to_string(), serde_json::json!(content_hash));
                object.insert("hash_algorithm".to_string(), serde_json::json!("sha256"));
                object.insert(
                    "text_index_version".to_string(),
                    serde_json::json!(RAG_TEXT_INDEX_VERSION),
                );
                object.insert(
                    "content_length".to_string(),
                    serde_json::json!(content_length),
                );
                object.insert(
                    "embedding_backend".to_string(),
                    serde_json::json!(embedding_identity.backend),
                );
                object.insert(
                    "embedding_model".to_string(),
                    serde_json::json!(embedding_identity.model),
                );
                object.insert(
                    "embedding_dimension".to_string(),
                    serde_json::json!(embedding_identity.dimension),
                );
                Ok(serde_json::Value::Object(object))
            }
            _ => Err(AppError::InvalidArgument(
                "RAG metadata 必须是 JSON 对象并包含项目族上下文".to_string(),
            )),
        }
    }
}

fn validate_project_family_metadata(
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
        None => {
            if root_project_id != project_id {
                return Err(AppError::InvalidArgument(format!(
                    "根文本 RAG metadata root_project_id 必须等于 project_id: project_id={project_id}, root_project_id={root_project_id}"
                )));
            }
        }
    }

    if source_snapshot_chapter_id.is_some()
        && (source_snapshot_id.is_none() || source_chapter_id.is_none())
    {
        return Err(AppError::InvalidArgument(
            "RAG metadata 设置 source_snapshot_chapter_id 时必须同时设置 source_snapshot_id 和 source_chapter_id"
                .to_string(),
        ));
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

fn content_sha256(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    format!("{:x}", hasher.finalize())
}
