use a4n_domain::ProviderError as AppError;

use crate::vector_store::EmbeddingRecord;

use super::records::{StoredRecord, validate_embedding_vector};
use super::{MemoryVectorStore, scoring};
use crate::errors::vector_store_error;

impl MemoryVectorStore {
    pub(super) fn add_memory_embeddings(
        &self,
        collection: &str,
        records: Vec<EmbeddingRecord>,
    ) -> Result<usize, AppError> {
        let count = records.len();
        let mut stored_records = Vec::with_capacity(count);
        let mut expected_dimension = self
            .collections
            .get(collection)
            .and_then(|coll| coll.iter().next().map(|record| record.embedding.len()));
        for record in records {
            let content = record.content;
            if content.trim().is_empty() {
                return Err(vector_store_error(format!(
                    "嵌入记录 content 不能为空：collection={collection}, id={}",
                    record.id
                )));
            }
            let dimension = validate_embedding_vector(collection, &record.id, &record.embedding)?;
            match expected_dimension {
                Some(expected) if expected != dimension => {
                    return Err(vector_store_error(format!(
                        "同一集合内嵌入向量维度不一致：collection={collection}, id={}, expected_dimension={expected}, actual_dimension={dimension}",
                        record.id
                    )));
                }
                Some(_) => {}
                None => expected_dimension = Some(dimension),
            }
            if scoring::tokenize(&content).is_empty() {
                return Err(vector_store_error(format!(
                    "嵌入记录无法生成 BM25 token：collection={collection}, id={}",
                    record.id
                )));
            }
            stored_records.push(StoredRecord {
                id: record.id,
                embedding: record.embedding,
                content,
                metadata: record.metadata,
            });
        }

        let to_index = {
            let coll = self.collections.entry(collection.to_string()).or_default();
            let mut ids = Vec::with_capacity(count);
            for stored in stored_records {
                ids.push((stored.id.clone(), stored.content.clone()));
                coll.insert(stored.id.clone(), stored);
            }
            ids
        };
        for (id, content) in to_index {
            self.bm25.index_document(collection, &id, &content)?;
        }
        Ok(count)
    }
}
