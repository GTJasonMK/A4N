use a4n_core::{error::AppError, rag::RAG_TEXT_INDEX_VERSION};
use a4n_embedding::provider::EmbeddingModelIdentity;
use sha2::{Digest, Sha256};

use crate::validation::required_content;

mod family;

#[cfg(test)]
mod tests;

pub fn content_hash(text: &str) -> Result<String, AppError> {
    let text = required_content(text)?;
    Ok(content_sha256(&text))
}

pub fn normalize_metadata(
    project_id: i64,
    text: &str,
    metadata: serde_json::Value,
    embedding_identity: &EmbeddingModelIdentity,
) -> Result<serde_json::Value, AppError> {
    let text = required_content(text)?;
    let content_hash = content_sha256(&text);
    let content_length = text.chars().count();
    match metadata {
        serde_json::Value::Object(mut object) => {
            family::validate_project_family_metadata(project_id, &object)?;
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

pub fn metadata_content_hash(metadata: &serde_json::Value) -> Option<&str> {
    metadata
        .get("content_hash")
        .and_then(serde_json::Value::as_str)
}

pub fn metadata_matches_current_text_index(metadata: &serde_json::Value) -> bool {
    metadata
        .get("text_index_version")
        .and_then(serde_json::Value::as_str)
        == Some(RAG_TEXT_INDEX_VERSION)
}

pub fn metadata_matches_embedding_identity(
    metadata: &serde_json::Value,
    identity: &EmbeddingModelIdentity,
) -> bool {
    let dimension = metadata
        .get("embedding_dimension")
        .and_then(serde_json::Value::as_u64)
        .and_then(|value| usize::try_from(value).ok());
    metadata
        .get("embedding_backend")
        .and_then(serde_json::Value::as_str)
        == Some(identity.backend.as_str())
        && metadata
            .get("embedding_model")
            .and_then(serde_json::Value::as_str)
            == Some(identity.model.as_str())
        && dimension == Some(identity.dimension)
}

fn content_sha256(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    format!("{:x}", hasher.finalize())
}
