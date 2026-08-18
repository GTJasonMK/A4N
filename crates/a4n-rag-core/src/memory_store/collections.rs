use std::collections::HashSet;

use crate::vector_store::DeleteFilter;

use super::{MemoryVectorStore, filters};

impl MemoryVectorStore {
    pub(super) fn delete_memory_records(&self, collection: &str, filter: DeleteFilter) -> usize {
        let coll = match self.collections.get(collection) {
            Some(collection) => collection,
            None => return 0,
        };
        let to_delete = coll
            .iter()
            .filter(|record| filters::matches_delete_filter(&record.id, &filter))
            .map(|record| record.id.clone())
            .collect::<Vec<_>>();
        let count = to_delete.len();
        let to_delete_set = to_delete.iter().cloned().collect::<HashSet<_>>();
        for id in &to_delete {
            coll.remove(id);
        }
        self.bm25.remove_documents(collection, &to_delete_set);
        count
    }

    pub(super) fn drop_memory_collection(&self, collection: &str) {
        self.collections.remove(collection);
        self.bm25.drop_collection(collection);
    }
}
