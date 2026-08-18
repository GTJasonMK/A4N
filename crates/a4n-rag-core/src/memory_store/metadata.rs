use std::collections::HashMap;

use crate::vector_store::DeleteFilter;

use super::{MemoryVectorStore, filters};

impl MemoryVectorStore {
    pub(super) fn memory_metadata_by_ids(
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

    pub(super) fn memory_metadata_by_filter(
        &self,
        collection: &str,
        filter: DeleteFilter,
    ) -> HashMap<String, serde_json::Value> {
        let Some(coll) = self.collections.get(collection) else {
            return HashMap::new();
        };
        coll.iter()
            .filter(|record| filters::matches_delete_filter(record.key(), &filter))
            .map(|record| (record.id.clone(), record.metadata.clone()))
            .collect()
    }
}
