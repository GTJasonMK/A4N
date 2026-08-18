use std::collections::HashMap;

use a4n_core::error::AppError;

use crate::vector_store::{SearchFilter, SearchResult};

use super::records::{checked_cosine_sim, vector_store_error};
use super::{MemoryVectorStore, filters, scoring};

impl MemoryVectorStore {
    pub(super) fn query_memory(
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
        for record in coll.iter().filter(|record| {
            filters
                .as_ref()
                .is_none_or(|filter| filters::matches_filter(&record.metadata, filter))
        }) {
            scored.push((
                checked_cosine_sim(collection, &record.id, &query_embedding, &record.embedding)?,
                record.clone(),
            ));
        }
        scoring::sort_scores_desc(&mut scored);
        Ok(scored
            .into_iter()
            .take(top_k)
            .map(|(score, record)| SearchResult {
                id: record.id,
                content: record.content,
                score,
                metadata: record.metadata,
            })
            .collect())
    }

    pub(super) fn search_memory_text(
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
        let mut doc_scores: HashMap<String, f32> = HashMap::new();
        for term in &query_terms {
            if let Some(postings) = self.bm25.postings(collection, term) {
                let doc_freq = postings.len();
                for (doc_id, tf) in postings.iter() {
                    let record = coll.get(doc_id).ok_or_else(|| {
                        vector_store_error(format!(
                            "BM25 索引引用了不存在的记录：collection={collection}, id={doc_id}"
                        ))
                    })?;
                    if filters
                        .as_ref()
                        .is_some_and(|filter| !filters::matches_filter(&record.metadata, filter))
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
            });
        }
        Ok(search_results)
    }

    pub(super) fn hybrid_memory_search(
        &self,
        collection: &str,
        query_text: &str,
        query_embedding: Vec<f32>,
        top_k: usize,
        filters: Option<SearchFilter>,
    ) -> Result<Vec<SearchResult>, AppError> {
        let vector_results =
            self.query_memory(collection, query_embedding, top_k * 2, filters.clone())?;
        let bm25_results = self.search_memory_text(collection, query_text, top_k * 2, filters)?;
        let coll = self
            .collections
            .get(collection)
            .ok_or_else(|| vector_store_error(format!("集合不存在：{collection}")))?;
        let mut results = Vec::with_capacity(top_k);
        for (id, score) in scoring::rrf_fuse_scores(&vector_results, &bm25_results, top_k) {
            let record = coll.get(&id).ok_or_else(|| {
                vector_store_error(format!(
                    "RRF 融合结果引用了不存在的记录：collection={collection}, id={id}"
                ))
            })?;
            results.push(SearchResult {
                id,
                content: record.content.clone(),
                score,
                metadata: record.metadata.clone(),
            });
        }
        Ok(results)
    }
}
