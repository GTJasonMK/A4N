use std::collections::HashMap;

use a4n_embedding::provider::EmbeddingModelIdentity;
use a4n_rag_core::metadata::{
    metadata_content_hash, metadata_matches_current_index_format,
    metadata_matches_current_text_index, metadata_matches_embedding_identity,
};
use serde_json::Value;

use super::contracts::{RagRecordSyncOutcome, RagSyncError};
use crate::OrchestratorContext;
use crate::rag::search::enhanced::validate_embedding_identity_match;

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

pub(super) async fn current_embedding_identity(
    context: &OrchestratorContext,
) -> Result<EmbeddingModelIdentity, RagSyncError> {
    context
        .embedding_model_identity()
        .await
        .map_err(|error| RagSyncError::ReadEmbeddingIdentity {
            message: error.to_string(),
        })
}

fn metadata_embedding_identity(metadata: &Value) -> Option<EmbeddingModelIdentity> {
    Some(EmbeddingModelIdentity {
        backend: metadata.get("embedding_backend")?.as_str()?.to_string(),
        model: metadata.get("embedding_model")?.as_str()?.to_string(),
        dimension: usize::try_from(metadata.get("embedding_dimension")?.as_u64()?).ok()?,
    })
}
