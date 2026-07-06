//! 内存向量存储：测试用
//!
//! 提供完整的 VectorStore trait 实现，包括向量搜索（余弦相似度）、
//! BM25 全文搜索和 RRF 混合搜索。所有数据存于内存，重启即丢失。

use async_trait::async_trait;
use dashmap::DashMap;
use std::collections::{HashMap, HashSet};

mod bm25;
mod filters;
mod scoring;
#[cfg(test)]
mod tests;

use super::vector_store::{
    CompletenessReport, DeleteFilter, EmbeddingRecord, SearchFilter, SearchResult, VectorStore,
};
use a4n_core::error::AppError;
use bm25::Bm25Index;

/// 集合内记录
#[derive(Debug, Clone)]
struct StoredRecord {
    id: String,
    embedding: Vec<f32>,
    content: String,
    metadata: serde_json::Value,
}

fn vector_store_error(message: impl Into<String>) -> AppError {
    AppError::VectorStoreError(message.into())
}

fn validate_embedding_vector(
    collection: &str,
    id: &str,
    embedding: &[f32],
) -> Result<usize, AppError> {
    if embedding.is_empty() {
        return Err(vector_store_error(format!(
            "嵌入向量不能为空：collection={collection}, id={id}"
        )));
    }
    if embedding.iter().any(|value| !value.is_finite()) {
        return Err(vector_store_error(format!(
            "嵌入向量包含非有限数：collection={collection}, id={id}"
        )));
    }
    Ok(embedding.len())
}

fn checked_cosine_sim(
    collection: &str,
    id: &str,
    query_embedding: &[f32],
    record_embedding: &[f32],
) -> Result<f32, AppError> {
    let query_dimension = validate_embedding_vector(collection, "<query>", query_embedding)?;
    let record_dimension = validate_embedding_vector(collection, id, record_embedding)?;
    if query_dimension != record_dimension {
        return Err(vector_store_error(format!(
            "嵌入向量维度不一致：collection={collection}, id={id}, query_dimension={query_dimension}, record_dimension={record_dimension}"
        )));
    }
    Ok(scoring::cosine_sim(query_embedding, record_embedding))
}

/// 内存向量存储
///
/// 线程安全，支持并发读写。数据结构：
/// - 向量存储：DashMap<collection -> DashMap<id -> StoredRecord>>
/// - BM25 索引：按 collection 分组的简单词频倒排索引
pub struct MemoryVectorStore {
    collections: DashMap<String, DashMap<String, StoredRecord>>,
    bm25: Bm25Index,
}

impl MemoryVectorStore {
    /// 创建空的内存向量存储
    pub fn new() -> Self {
        Self {
            collections: DashMap::new(),
            bm25: Bm25Index::new(),
        }
    }
}

impl Default for MemoryVectorStore {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl VectorStore for MemoryVectorStore {
    async fn add_embeddings(
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
            let content = match record.content {
                Some(content) if !content.trim().is_empty() => content,
                Some(_) => {
                    return Err(vector_store_error(format!(
                        "嵌入记录 content 不能为空：collection={collection}, id={}",
                        record.id
                    )));
                }
                None => {
                    return Err(vector_store_error(format!(
                        "嵌入记录缺少 content：collection={collection}, id={}",
                        record.id
                    )));
                }
            };
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

        // 先插入所有记录（持有写锁期间不做索引操作，避免死锁）
        let to_index: Vec<(String, String)> = {
            let coll = self.collections.entry(collection.to_string()).or_default();

            let mut ids = Vec::with_capacity(count);
            for stored in stored_records {
                ids.push((stored.id.clone(), stored.content.clone()));
                coll.insert(stored.id.clone(), stored);
            }
            ids
        }; // 写锁在此释放

        // 释放写锁后再更新 BM25 索引
        for (id, content) in to_index {
            self.bm25.index_document(collection, &id, &content)?;
        }

        Ok(count)
    }

    async fn query(
        &self,
        collection: &str,
        query_embedding: Vec<f32>,
        top_k: usize,
        filters: Option<SearchFilter>,
    ) -> Result<Vec<SearchResult>, AppError> {
        let coll = self
            .collections
            .get(collection)
            .ok_or_else(|| vector_store_error(format!("集合不存在：{collection}")))?;

        let mut scored = Vec::new();
        for r in coll.iter().filter(|r| {
            if let Some(ref f) = filters {
                filters::matches_filter(&r.metadata, f)
            } else {
                true
            }
        }) {
            scored.push((
                checked_cosine_sim(collection, &r.id, &query_embedding, &r.embedding)?,
                r.clone(),
            ));
        }

        scoring::sort_scores_desc(&mut scored);

        Ok(scored
            .into_iter()
            .take(top_k)
            .map(|(score, r)| SearchResult {
                id: r.id,
                content: r.content,
                score,
                vector_score: Some(score),
                bm25_score: None,
                metadata: r.metadata,
            })
            .collect())
    }

    async fn search_text(
        &self,
        collection: &str,
        query_text: &str,
        top_k: usize,
        filters: Option<SearchFilter>,
    ) -> Result<Vec<SearchResult>, AppError> {
        let coll = self
            .collections
            .get(collection)
            .ok_or_else(|| vector_store_error(format!("集合不存在：{collection}")))?;

        let query_terms = scoring::tokenize(query_text);
        let total_docs = coll.len();

        // 计算每个文档的 BM25 总分
        let mut doc_scores: HashMap<String, f32> = HashMap::new();
        for term in &query_terms {
            if let Some(postings) = self.bm25.postings(collection, term) {
                let doc_freq = postings.len();
                for (doc_id, tf) in postings.iter() {
                    // 检查过滤条件
                    let record = coll.get(doc_id).ok_or_else(|| {
                        vector_store_error(format!(
                            "BM25 索引引用了不存在的记录：collection={collection}, id={doc_id}"
                        ))
                    })?;
                    if let Some(ref f) = filters
                        && !filters::matches_filter(&record.metadata, f)
                    {
                        continue;
                    }
                    let score = self
                        .bm25
                        .score(doc_id, collection, *tf, total_docs, doc_freq)?;
                    *doc_scores.entry(doc_id.clone()).or_insert(0.0) += score;
                }
            }
        }

        let mut results: Vec<(String, f32)> = doc_scores.into_iter().collect();
        scoring::sort_doc_scores_desc(&mut results);

        let mut search_results = Vec::with_capacity(results.len().min(top_k));
        for (id, score) in results.into_iter().take(top_k) {
            let record = coll.get(&id).ok_or_else(|| {
                vector_store_error(format!(
                    "BM25 搜索结果引用了不存在的记录：collection={collection}, id={id}"
                ))
            })?;
            search_results.push(SearchResult {
                content: record.content.clone(),
                metadata: record.metadata.clone(),
                id,
                score,
                vector_score: None,
                bm25_score: Some(score),
            });
        }

        Ok(search_results)
    }

    async fn hybrid_search(
        &self,
        collection: &str,
        query_text: &str,
        query_embedding: Vec<f32>,
        top_k: usize,
        filters: Option<SearchFilter>,
        _vector_weight: Option<f32>,
    ) -> Result<Vec<SearchResult>, AppError> {
        // 同时执行向量搜索和 BM25 搜索，然后 RRF 融合
        let vector_results = self
            .query(collection, query_embedding, top_k * 2, filters.clone())
            .await?;
        let bm25_results = self
            .search_text(collection, query_text, top_k * 2, filters)
            .await?;

        let coll = self
            .collections
            .get(collection)
            .ok_or_else(|| vector_store_error(format!("集合不存在：{collection}")))?;
        let mut results = Vec::with_capacity(top_k);
        for fused in scoring::rrf_fuse_scores(&vector_results, &bm25_results, top_k, 60) {
            let record = coll.get(&fused.id).ok_or_else(|| {
                vector_store_error(format!(
                    "RRF 融合结果引用了不存在的记录：collection={collection}, id={}",
                    fused.id
                ))
            })?;
            results.push(SearchResult {
                id: fused.id,
                content: record.content.clone(),
                score: fused.score,
                vector_score: Some(fused.vector_score),
                bm25_score: Some(fused.bm25_score),
                metadata: record.metadata.clone(),
            });
        }

        Ok(results)
    }

    async fn delete(&self, collection: &str, filter: DeleteFilter) -> Result<usize, AppError> {
        let coll = match self.collections.get(collection) {
            Some(c) => c,
            None => return Ok(0),
        };

        let to_delete: Vec<String> = coll
            .iter()
            .filter(|r| filters::matches_delete_filter(&r.id, &r.metadata, &filter))
            .map(|r| r.id.clone())
            .collect();

        let count = to_delete.len();
        let to_delete_set = to_delete.iter().cloned().collect::<HashSet<_>>();
        for id in &to_delete {
            coll.remove(id);
        }

        self.bm25.remove_documents(collection, &to_delete_set);

        Ok(count)
    }

    async fn collection_exists(&self, collection: &str) -> Result<bool, AppError> {
        Ok(self.collections.contains_key(collection))
    }

    async fn create_collection(&self, collection: &str) -> Result<(), AppError> {
        self.collections.entry(collection.to_string()).or_default();
        Ok(())
    }

    async fn count(&self, collection: &str) -> Result<usize, AppError> {
        self.collections
            .get(collection)
            .map(|c| c.len())
            .ok_or_else(|| vector_store_error(format!("集合不存在：{collection}")))
    }

    async fn drop_collection(&self, collection: &str) -> Result<(), AppError> {
        self.collections.remove(collection);
        self.bm25.drop_collection(collection);
        Ok(())
    }

    async fn list_collections(&self) -> Result<Vec<String>, AppError> {
        Ok(self.collections.iter().map(|e| e.key().clone()).collect())
    }

    async fn update_metadata(
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

    async fn metadata_by_ids(
        &self,
        collection: &str,
        ids: &[String],
    ) -> Result<HashMap<String, serde_json::Value>, AppError> {
        if ids.is_empty() {
            return Ok(HashMap::new());
        }
        let Some(coll) = self.collections.get(collection) else {
            return Ok(HashMap::new());
        };

        Ok(ids
            .iter()
            .filter_map(|id| {
                coll.get(id)
                    .map(|record| (id.clone(), record.metadata.clone()))
            })
            .collect())
    }

    async fn metadata_by_filter(
        &self,
        collection: &str,
        filter: DeleteFilter,
    ) -> Result<HashMap<String, serde_json::Value>, AppError> {
        let Some(coll) = self.collections.get(collection) else {
            return Ok(HashMap::new());
        };

        Ok(coll
            .iter()
            .filter(|record| {
                filters::matches_delete_filter(record.key(), &record.metadata, &filter)
            })
            .map(|record| (record.id.clone(), record.metadata.clone()))
            .collect())
    }

    async fn health_check(&self) -> Result<bool, AppError> {
        Ok(true)
    }

    async fn completeness_report(
        &self,
        collection: &str,
        expected_count: usize,
    ) -> Result<CompletenessReport, AppError> {
        let actual = self
            .collections
            .get(collection)
            .map(|c| c.len())
            .ok_or_else(|| vector_store_error(format!("集合不存在：{collection}")))?;

        // 统计缺失 ID
        let missing: Vec<String> = if actual < expected_count {
            (actual..expected_count)
                .map(|i| format!("missing-{i}"))
                .collect()
        } else {
            Vec::new()
        };

        Ok(CompletenessReport::count_only(
            collection.to_string(),
            expected_count,
            actual,
            actual,
            missing.clone(),
            missing,
        ))
    }

    async fn completeness_report_by_ids(
        &self,
        collection: &str,
        expected_ids: &[String],
    ) -> Result<CompletenessReport, AppError> {
        let Some(actual_ids) = self.collections.get(collection).map(|c| {
            c.iter()
                .map(|entry| entry.key().clone())
                .collect::<HashSet<_>>()
        }) else {
            return Ok(CompletenessReport::from_missing_ids(
                collection.to_string(),
                expected_ids.len(),
                0,
                0,
                expected_ids.to_vec(),
                expected_ids.to_vec(),
            ));
        };

        Ok(CompletenessReport::by_expected_ids(
            collection.to_string(),
            expected_ids,
            &actual_ids,
            Some(&actual_ids),
        ))
    }
}
