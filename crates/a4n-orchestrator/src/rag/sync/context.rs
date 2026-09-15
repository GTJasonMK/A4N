use std::collections::{HashMap, HashSet};

use a4n_domain::rag::RagDataType;
use a4n_embedding_provider_contract::EmbeddingModelIdentity;
use a4n_rag_core::{metadata::content_hash, vector_store::DeleteFilter};
use serde_json::Value;

use super::identity::current_embedding_identity;
use crate::{OrchestratorContext, rag::rag_project_prefix};
use a4n_rag_core::contracts::sync::classify_rag_record;
use a4n_rag_core::{
    DeleteRagRecordsInput, PreparedRagRecordSync, RagRecord, RagRecordSyncOutcome, RagSyncError,
};

pub struct RagSyncContext {
    pub(super) project_id: i64,
    pub(super) embedding_identity: EmbeddingModelIdentity,
    pub(super) stored_metadata: HashMap<String, Value>,
    pub(super) stale_ids: HashSet<String>,
}

fn rag_id_prefix(project_id: i64, data_type: RagDataType) -> Result<String, RagSyncError> {
    match data_type {
        RagDataType::Chapter
        | RagDataType::ChapterSummary
        | RagDataType::ChapterMetadata
        | RagDataType::KeyEvent => Err(RagSyncError::InvalidRecord {
            message: format!(
                "{} 需要按具体章节清理，不能按共享章节前缀清理",
                data_type.as_str()
            ),
        }),
        _ => Ok(rag_project_prefix(project_id, data_type)),
    }
}

impl RagSyncContext {
    pub async fn for_project(
        context: &OrchestratorContext,
        project_id: i64,
    ) -> Result<Self, RagSyncError> {
        Self::for_filter(
            context,
            project_id,
            DeleteFilter {
                ids: Vec::new(),
                id_prefixes: Vec::new(),
            },
        )
        .await
    }

    pub(crate) async fn for_ids(
        context: &OrchestratorContext,
        project_id: i64,
        ids: &[String],
    ) -> Result<Self, RagSyncError> {
        let stored_metadata = context
            .rag_project_metadata_by_ids(project_id, ids)
            .await
            .map_err(|error| RagSyncError::ReadStoredMetadata {
                message: error.to_string(),
            })?;
        let embedding_identity = current_embedding_identity(context).await?;
        Ok(Self {
            project_id,
            embedding_identity,
            stored_metadata,
            stale_ids: HashSet::new(),
        })
    }

    pub(crate) async fn for_id_prefixes(
        context: &OrchestratorContext,
        project_id: i64,
        prefixes: Vec<String>,
    ) -> Result<Self, RagSyncError> {
        Self::for_filter(
            context,
            project_id,
            DeleteFilter {
                ids: Vec::new(),
                id_prefixes: prefixes,
            },
        )
        .await
    }

    pub async fn for_data_types(
        context: &OrchestratorContext,
        project_id: i64,
        data_types: &[RagDataType],
    ) -> Result<Self, RagSyncError> {
        if data_types.is_empty() {
            return Err(RagSyncError::InvalidRecord {
                message: "RAG 同步 data_types 不能为空".to_string(),
            });
        }
        let prefixes = data_types
            .iter()
            .copied()
            .map(|data_type| rag_id_prefix(project_id, data_type))
            .collect::<Result<Vec<_>, _>>()?;
        Self::for_id_prefixes(context, project_id, prefixes).await
    }

    async fn for_filter(
        context: &OrchestratorContext,
        project_id: i64,
        filter: DeleteFilter,
    ) -> Result<Self, RagSyncError> {
        let stored_metadata = context
            .rag_project_metadata_by_filter(project_id, filter)
            .await
            .map_err(|error| RagSyncError::ReadStoredMetadata {
                message: error.to_string(),
            })?;
        let stale_ids = stored_metadata.keys().cloned().collect();
        let embedding_identity = current_embedding_identity(context).await?;
        Ok(Self {
            project_id,
            embedding_identity,
            stored_metadata,
            stale_ids,
        })
    }

    pub(crate) fn into_port_snapshot(
        self,
    ) -> (EmbeddingModelIdentity, HashMap<String, Value>, Vec<String>) {
        let mut stale_ids = self.stale_ids.into_iter().collect::<Vec<_>>();
        stale_ids.sort();
        (self.embedding_identity, self.stored_metadata, stale_ids)
    }

    pub fn prepare_record(
        &mut self,
        record: RagRecord,
    ) -> Result<PreparedRagRecordSync, RagSyncError> {
        let content_hash =
            content_hash(&record.text).map_err(|error| RagSyncError::InvalidRecord {
                message: error.to_string(),
            })?;
        let outcome = classify_rag_record(
            self.stored_metadata.get(&record.chunk_id),
            &content_hash,
            &self.embedding_identity,
        );
        self.stale_ids.remove(&record.chunk_id);
        Ok(PreparedRagRecordSync::new(record, outcome))
    }

    pub(crate) async fn sync_record(
        &mut self,
        context: &OrchestratorContext,
        record: RagRecord,
    ) -> Result<RagRecordSyncOutcome, RagSyncError> {
        let prepared = self.prepare_record(record)?;
        let (record, outcome) = prepared.into_parts();
        if outcome != RagRecordSyncOutcome::Skipped {
            context
                .rag_embed_and_store_record(
                    self.project_id,
                    &record.chunk_id,
                    &record.text,
                    record.metadata,
                )
                .await
                .map_err(|error| RagSyncError::Store {
                    message: error.to_string(),
                })?;
        }
        Ok(outcome)
    }

    pub async fn cleanup_stale(self, context: &OrchestratorContext) -> Result<u32, RagSyncError> {
        if self.stale_ids.is_empty() {
            return Ok(0);
        }
        let mut ids = self.stale_ids.into_iter().collect::<Vec<_>>();
        ids.sort();
        let deleted = context
            .delete_rag_records(DeleteRagRecordsInput {
                project_id: self.project_id,
                ids,
                id_prefixes: Vec::new(),
            })
            .await
            .map_err(|error| RagSyncError::DeleteStale {
                message: error.to_string(),
            })?;
        u32::try_from(deleted).map_err(|_| RagSyncError::DeletedCountOverflow { deleted })
    }
}
