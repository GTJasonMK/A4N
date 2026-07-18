use std::collections::HashMap;

use a4n_core::rag::RagDataType;
use a4n_rag_core::vector_store::{DeleteFilter, VectorStore};
use serde_json::Value;

use super::contracts::{
    DeleteRagRecordsInput, PreparedRagRecordSync, RagRecordSyncOutcome, RagSyncError,
};
use super::keys::{
    chapter_rag_delete_filter, character_attribute_event_rag_chunk_id,
    character_state_rag_chunk_id, foreshadowing_rag_chunk_id, protagonist_rag_chunk_prefix,
};
use crate::OrchestratorContext;

impl OrchestratorContext {
    pub async fn clear_project_rag(&self, project_id: i64) -> Result<(), RagSyncError> {
        clear_project_rag_store(self.rag_store.as_ref(), project_id).await
    }

    pub async fn delete_rag_records(
        &self,
        input: DeleteRagRecordsInput,
    ) -> Result<usize, RagSyncError> {
        if input.ids.is_empty() && input.id_prefixes.is_empty() {
            return Ok(0);
        }
        let project_id = input.project_id;
        self.delete_rag_records_with_filter(
            project_id,
            DeleteFilter {
                project_id,
                data_type: input.data_type.map(|value| value.as_str().to_string()),
                chapter_number: None,
                ids: input.ids,
                id_prefixes: input.id_prefixes,
            },
        )
        .await
    }

    pub async fn delete_chapter_rag_by_id(
        &self,
        project_id: i64,
        chapter_id: i64,
        chapter_number: i32,
    ) -> Result<usize, RagSyncError> {
        let filter = chapter_rag_delete_filter(project_id, chapter_id, chapter_number)?;
        self.delete_rag_records_with_filter(project_id, filter)
            .await
            .map_err(|error| match error {
                RagSyncError::Delete { message } => RagSyncError::DeleteChapter { message },
                error => error,
            })
    }

    async fn delete_rag_records_with_filter(
        &self,
        project_id: i64,
        filter: DeleteFilter,
    ) -> Result<usize, RagSyncError> {
        self.rag_store
            .delete(&project_id.to_string(), filter)
            .await
            .map_err(|error| RagSyncError::Delete {
                message: error.to_string(),
            })
    }

    pub async fn delete_character_state_rag_by_ids(
        &self,
        project_id: i64,
        character_state_ids: &[i64],
    ) -> Result<usize, RagSyncError> {
        self.delete_rag_records(DeleteRagRecordsInput {
            project_id,
            data_type: Some(RagDataType::CharacterState),
            ids: character_state_ids
                .iter()
                .map(|id| character_state_rag_chunk_id(project_id, *id))
                .collect(),
            id_prefixes: Vec::new(),
        })
        .await
    }

    pub async fn delete_character_attribute_event_rag_by_ids(
        &self,
        project_id: i64,
        attribute_event_ids: &[i64],
    ) -> Result<usize, RagSyncError> {
        self.delete_rag_records(DeleteRagRecordsInput {
            project_id,
            data_type: Some(RagDataType::CharacterAttributeEvent),
            ids: attribute_event_ids
                .iter()
                .map(|id| character_attribute_event_rag_chunk_id(project_id, *id))
                .collect(),
            id_prefixes: Vec::new(),
        })
        .await
    }

    pub async fn delete_protagonist_rag_by_ids(
        &self,
        project_id: i64,
        profile_ids: &[i64],
    ) -> Result<usize, RagSyncError> {
        self.delete_rag_records(DeleteRagRecordsInput {
            project_id,
            data_type: Some(RagDataType::Protagonist),
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
            data_type: Some(RagDataType::Foreshadowing),
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
        self.rag_store
            .metadata_by_filter(
                &project_id.to_string(),
                DeleteFilter {
                    project_id,
                    data_type: None,
                    chapter_number: None,
                    ids: Vec::new(),
                    id_prefixes: Vec::new(),
                },
            )
            .await
            .map_err(|error| RagSyncError::ReadStoredMetadata {
                message: error.to_string(),
            })
    }

    pub async fn store_prepared_rag_records<'a, I>(
        &self,
        project_id: i64,
        records: I,
    ) -> Result<(), RagSyncError>
    where
        I: IntoIterator<Item = &'a PreparedRagRecordSync>,
    {
        let inputs = records
            .into_iter()
            .filter(|prepared| prepared.outcome != RagRecordSyncOutcome::Skipped)
            .map(PreparedRagRecordSync::embedding_input)
            .collect::<Vec<_>>();
        if inputs.is_empty() {
            return Ok(());
        }
        self.embed_and_store_rag_records(project_id, inputs)
            .await
            .map_err(|error| RagSyncError::Store {
                message: error.to_string(),
            })
            .map(|_| ())
    }
}

pub(super) async fn clear_project_rag_store(
    vector_store: &dyn VectorStore,
    project_id: i64,
) -> Result<(), RagSyncError> {
    vector_store
        .drop_collection(&project_id.to_string())
        .await
        .map_err(|error| RagSyncError::ClearProject {
            message: error.to_string(),
        })
}
