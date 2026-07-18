use std::collections::HashSet;

use a4n_core::error::AppError;

use crate::vector_store::DeleteFilter;

use super::records::vector_store_error;
use super::{MemoryVectorStore, filters};

impl MemoryVectorStore {
    pub(super) async fn delete_memory_records(
        &self,
        collection: &str,
        filter: DeleteFilter,
    ) -> Result<usize, AppError> {
        let coll = match self.collections.get(collection) {
            Some(collection) => collection,
            None => return Ok(0),
        };
        let to_delete = coll
            .iter()
            .filter(|record| filters::matches_delete_filter(&record.id, &record.metadata, &filter))
            .map(|record| record.id.clone())
            .collect::<Vec<_>>();
        let count = to_delete.len();
        let to_delete_set = to_delete.iter().cloned().collect::<HashSet<_>>();
        for id in &to_delete {
            coll.remove(id);
        }
        self.bm25.remove_documents(collection, &to_delete_set);
        Ok(count)
    }

    pub(super) async fn memory_collection_exists(&self, collection: &str) -> bool {
        self.collections.contains_key(collection)
    }

    pub(super) async fn memory_collection_count(
        &self,
        collection: &str,
    ) -> Result<usize, AppError> {
        self.collections
            .get(collection)
            .map(|collection| collection.len())
            .ok_or_else(|| vector_store_error(format!("集合不存在：{collection}")))
    }

    pub(super) async fn drop_memory_collection(&self, collection: &str) {
        self.collections.remove(collection);
        self.bm25.drop_collection(collection);
    }

    pub(super) async fn list_memory_collections(&self) -> Vec<String> {
        self.collections
            .iter()
            .map(|entry| entry.key().clone())
            .collect()
    }
}
