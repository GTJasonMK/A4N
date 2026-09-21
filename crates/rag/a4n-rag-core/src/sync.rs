//! RAG record synchronization rules and errors.

use std::collections::HashMap;

use a4n_embedding_provider_contract::EmbeddingModelIdentity;
use serde_json::Value;

use crate::{
    metadata::{
        metadata_content_hash, metadata_matches_current_index_format,
        metadata_matches_current_text_index, metadata_matches_embedding_identity,
    },
    validation::validate_embedding_identity_match,
};

use a4n_rag_value_contract::sync::RagRecordSyncOutcome;

#[derive(Debug, thiserror::Error)]
pub enum RagSyncError {
    #[error("{message}")]
    InvalidRecord { message: String },
    #[error("读取 RAG 已同步记录失败: {message}")]
    ReadStoredMetadata { message: String },
    #[error("读取当前嵌入模型身份失败: {message}")]
    ReadEmbeddingIdentity { message: String },
    #[error("{message}")]
    Store { message: String },
    #[error("{message}")]
    Delete { message: String },
    #[error("清理过期 RAG 记录失败: {message}")]
    DeleteStale { message: String },
    #[error("清理过期 RAG 记录数量超过 u32 上限：{deleted}")]
    DeletedCountOverflow { deleted: usize },
    #[error("{message}")]
    EmbeddingIdentityMismatch { message: String },
}

pub struct DeleteRagRecordsInput {
    pub project_id: i64,
    pub ids: Vec<String>,
    pub id_prefixes: Vec<String>,
}

pub fn classify_rag_record(
    stored: Option<&Value>,
    content_hash: &str,
    embedding_identity: &EmbeddingModelIdentity,
) -> RagRecordSyncOutcome {
    let already_synced = stored.is_some_and(|metadata| {
        metadata_content_hash(metadata).is_some_and(|stored_hash| stored_hash == content_hash)
            && metadata_matches_current_index_format(metadata)
            && metadata_matches_current_text_index(metadata)
            && metadata_matches_embedding_identity(metadata, embedding_identity)
    });
    if already_synced {
        RagRecordSyncOutcome::Skipped
    } else if stored.is_some() {
        RagRecordSyncOutcome::Updated
    } else {
        RagRecordSyncOutcome::Added
    }
}

pub fn ensure_project_embedding_identity_matches(
    stored_metadata: &HashMap<String, Value>,
    expected: &EmbeddingModelIdentity,
) -> Result<(), RagSyncError> {
    for metadata in stored_metadata.values() {
        let Some(actual) = metadata_embedding_identity(metadata) else {
            continue;
        };
        if &actual != expected {
            return Err(RagSyncError::EmbeddingIdentityMismatch {
                message: format!(
                    "项目 RAG embedding identity 已锁定为 {} / {} / {} 维，当前本机配置为 {} / {} / {} 维；请切回项目模型或显式重建 RAG",
                    actual.backend,
                    actual.model,
                    actual.dimension,
                    expected.backend,
                    expected.model,
                    expected.dimension
                ),
            });
        }
    }
    Ok(())
}

pub fn ensure_embedding_identity_matches(
    actual: &EmbeddingModelIdentity,
    expected: &EmbeddingModelIdentity,
) -> Result<(), RagSyncError> {
    validate_embedding_identity_match(actual, expected).map_err(|error| {
        RagSyncError::EmbeddingIdentityMismatch {
            message: error.to_string(),
        }
    })
}

fn metadata_embedding_identity(metadata: &Value) -> Option<EmbeddingModelIdentity> {
    Some(EmbeddingModelIdentity {
        backend: metadata.get("embedding_backend")?.as_str()?.to_string(),
        model: metadata.get("embedding_model")?.as_str()?.to_string(),
        dimension: usize::try_from(metadata.get("embedding_dimension")?.as_u64()?).ok()?,
    })
}
