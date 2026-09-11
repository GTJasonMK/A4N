use std::collections::HashMap;

use a4n_rag_core::vector_store::DeleteFilter;
use serde_json::Value;

use crate::{
    OrchestratorContext,
    rag::{
        chapter_rag_chunk_prefix, character_attribute_event_rag_chunk_id,
        character_state_rag_chunk_id, foreshadowing_rag_chunk_id, protagonist_rag_chunk_prefix,
    },
};
use a4n_rag_core::{
    DeleteRagRecordsInput, PreparedRagRecordSync, RagRecordSyncOutcome, RagSyncError,
};

impl OrchestratorContext {
    pub async fn clear_project_rag(&self, project_id: i64) -> Result<(), RagSyncError> {
        self.rag_drop_project_collection(project_id)
            .await
            .map_err(|error| RagSyncError::Delete {
                message: error.to_string(),
            })
    }

    pub async fn delete_rag_records(
        &self,
        input: DeleteRagRecordsInput,
    ) -> Result<usize, RagSyncError> {
        if input.ids.is_empty() && input.id_prefixes.is_empty() {
            return Ok(0);
        }
        let project_id = input.project_id;
        self.rag_delete_project_records(
            project_id,
            DeleteFilter {
                ids: input.ids,
                id_prefixes: input.id_prefixes,
            },
        )
        .await
        .map_err(|error| RagSyncError::Delete {
            message: error.to_string(),
        })
    }

    pub(crate) async fn delete_chapter_rag_by_id(
        &self,
        project_id: i64,
        chapter_id: i64,
    ) -> Result<usize, RagSyncError> {
        self.delete_rag_records(DeleteRagRecordsInput {
            project_id,
            ids: Vec::new(),
            id_prefixes: vec![chapter_rag_chunk_prefix(project_id, chapter_id)],
        })
        .await
    }

    pub(crate) async fn delete_character_state_rag_by_ids(
        &self,
        project_id: i64,
        character_state_ids: &[i64],
    ) -> Result<usize, RagSyncError> {
        self.delete_rag_records(DeleteRagRecordsInput {
            project_id,
            ids: character_state_ids
                .iter()
                .map(|id| character_state_rag_chunk_id(project_id, *id))
                .collect(),
            id_prefixes: Vec::new(),
        })
        .await
    }

    pub(crate) async fn delete_character_attribute_event_rag_by_ids(
        &self,
        project_id: i64,
        attribute_event_ids: &[i64],
    ) -> Result<usize, RagSyncError> {
        self.delete_rag_records(DeleteRagRecordsInput {
            project_id,
            ids: attribute_event_ids
                .iter()
                .map(|id| character_attribute_event_rag_chunk_id(project_id, *id))
                .collect(),
            id_prefixes: Vec::new(),
        })
        .await
    }

    pub(crate) async fn delete_protagonist_rag_by_ids(
        &self,
        project_id: i64,
        profile_ids: &[i64],
    ) -> Result<usize, RagSyncError> {
        self.delete_rag_records(DeleteRagRecordsInput {
            project_id,
            ids: Vec::new(),
            id_prefixes: profile_ids
                .iter()
                .map(|id| protagonist_rag_chunk_prefix(project_id, *id))
                .collect(),
        })
        .await
    }

    pub async fn delete_foreshadowing_rag_by_ids(
        &self,
        project_id: i64,
        foreshadowing_ids: &[i64],
    ) -> Result<usize, RagSyncError> {
        self.delete_rag_records(DeleteRagRecordsInput {
            project_id,
            ids: foreshadowing_ids
                .iter()
                .map(|id| foreshadowing_rag_chunk_id(project_id, *id))
                .collect(),
            id_prefixes: Vec::new(),
        })
        .await
    }

    pub async fn project_rag_metadata(
        &self,
        project_id: i64,
    ) -> Result<HashMap<String, Value>, RagSyncError> {
        self.rag_project_metadata_by_filter(
            project_id,
            DeleteFilter {
                ids: Vec::new(),
                id_prefixes: Vec::new(),
            },
        )
        .await
        .map_err(|error| RagSyncError::ReadStoredMetadata {
            message: error.to_string(),
        })
    }

    pub async fn store_prepared_rag_records<I>(
        &self,
        project_id: i64,
        records: I,
    ) -> Result<(), RagSyncError>
    where
        I: IntoIterator<Item = PreparedRagRecordSync>,
    {
        let inputs = records
            .into_iter()
            .filter(|prepared| prepared.outcome() != RagRecordSyncOutcome::Skipped)
            .map(|prepared| {
                let (record, _) = prepared.into_parts();
                super::super::write::RagEmbeddingInput {
                    chunk_id: record.chunk_id,
                    text: record.text,
                    metadata: record.metadata,
                }
            })
            .collect::<Vec<_>>();
        if inputs.is_empty() {
            return Ok(());
        }
        self.rag_embed_and_store_records(project_id, inputs)
            .await
            .map_err(|error| RagSyncError::Store {
                message: error.to_string(),
            })
            .map(|_| ())
    }
}
