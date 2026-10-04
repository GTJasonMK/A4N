use a4n_db::rag::RagForeshadowingRow;
use a4n_domain::character::CharacterAttributeEventRecord;
use a4n_rag_core::RagRecord;

use super::{
    attributes::character_attribute_event_rag_record,
    foreshadowing::foreshadowing_rag_record_from_row,
};
use crate::{OrchestratorContext, rag::RagSyncContext};

impl OrchestratorContext {
    pub(crate) async fn sync_character_attribute_event_rag(
        &self,
        project_id: i64,
        event: &CharacterAttributeEventRecord,
        character_name: &str,
    ) -> Result<(), String> {
        let record = character_attribute_event_rag_record(project_id, event, character_name)?;
        self.sync_single_rag_record(project_id, record).await
    }

    pub async fn sync_foreshadowing_rag(
        &self,
        project_id: i64,
        item: &RagForeshadowingRow,
    ) -> Result<(), String> {
        let record = foreshadowing_rag_record_from_row(project_id, item)?;
        self.sync_single_rag_record(project_id, record).await
    }

    pub(crate) async fn reingest_foreshadowing_rag_by_ids(
        &self,
        project_id: i64,
        foreshadowing_ids: &[i64],
    ) -> Result<(), String> {
        if foreshadowing_ids.is_empty() {
            return Ok(());
        }
        let items = self
            .foreshadowing_items_by_ids(project_id, foreshadowing_ids)
            .await
            .map_err(|error| format!("查询需更新的伏笔 RAG 失败: {error}"))?;
        let removed: Vec<_> = foreshadowing_ids
            .iter()
            .copied()
            .filter(|id| !items.iter().any(|item| item.id == *id))
            .collect();
        self.delete_foreshadowing_rag_by_ids(project_id, &removed)
            .await
            .map_err(|error| format!("清理已撤回的伏笔 RAG 失败: {error}"))?;
        if items.is_empty() {
            return Ok(());
        }
        let records = items
            .iter()
            .map(|item| foreshadowing_rag_record_from_row(project_id, item))
            .collect::<Result<Vec<_>, _>>()?;
        let ids = records
            .iter()
            .map(|record| record.chunk_id.clone())
            .collect::<Vec<_>>();
        let mut sync = RagSyncContext::for_ids(self, project_id, &ids)
            .await
            .map_err(|error| error.to_string())?;
        let prepared = records
            .into_iter()
            .map(|record| sync.prepare_record(record))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| error.to_string())?;
        self.store_prepared_rag_records(project_id, prepared)
            .await
            .map_err(|error| error.to_string())?;
        Ok(())
    }

    async fn sync_single_rag_record(
        &self,
        project_id: i64,
        record: RagRecord,
    ) -> Result<(), String> {
        let mut sync =
            RagSyncContext::for_ids(self, project_id, std::slice::from_ref(&record.chunk_id))
                .await
                .map_err(|error| error.to_string())?;
        sync.sync_record(self, record)
            .await
            .map(|_| ())
            .map_err(|error| error.to_string())
    }
}
