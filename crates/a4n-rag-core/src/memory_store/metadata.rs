use std::collections::HashMap;

use a4n_core::error::AppError;

use crate::vector_store::DeleteFilter;

use super::records::vector_store_error;
use super::{MemoryVectorStore, filters};

impl MemoryVectorStore {
    pub(super) async fn update_memory_metadata(
        &self,
        collection: &str,
        id: &str,
        metadata: serde_json::Value,
    ) -> Result<(), AppError> {
        let coll = self
            .collections
            .get(collection)
            .ok_or_else(|| vector_store_error(format!("集合不存在：{collection}")))?;
        let mut record = coll.get_mut(id).ok_or_else(|| {
            vector_store_error(format!("集合记录不存在：collection={collection}, id={id}"))
        })?;
        record.metadata = metadata;
        Ok(())
    }

    pub(super) async fn memory_metadata_by_ids(
        &self,
        collection: &str,
        ids: &[String],
    ) -> HashMap<String, serde_json::Value> {
        if ids.is_empty() {
            return HashMap::new();
        }
        let Some(coll) = self.collections.get(collection) else {
            return HashMap::new();
        };
        ids.iter()
            .filter_map(|id| {
                coll.get(id)
                    .map(|record| (id.clone(), record.metadata.clone()))
            })
            .collect()
    }

    pub(super) async fn memory_metadata_by_filter(
        &self,
        collection: &str,
        filter: DeleteFilter,
    ) -> HashMap<String, serde_json::Value> {
        let Some(coll) = self.collections.get(collection) else {
            return HashMap::new();
        };
        coll.iter()
            .filter(|record| {
                filters::matches_delete_filter(record.key(), &record.metadata, &filter)
            })
            .map(|record| (record.id.clone(), record.metadata.clone()))
            .collect()
    }
}
