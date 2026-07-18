use a4n_core::error::AppError;
use a4n_rag_core::vector_store::{SearchFilter, SearchResult};
use tantivy::TantivyDocument;
use tantivy::collector::TopDocs;

use super::document::{
    optional_i64_field, optional_text_field, required_i64_field, required_text_field,
};
use super::query::{build_search_query, collection_excluded_by_filter, ngram_query_terms};
use super::state::TantivyTextIndex;

impl TantivyTextIndex {
    pub async fn search(
        &self,
        collection: &str,
        query_text: &str,
        top_k: usize,
    ) -> Result<Vec<SearchResult>, AppError> {
        self.search_filtered(collection, query_text, top_k, None)
            .await
    }

    pub async fn search_filtered(
        &self,
        collection: &str,
        query_text: &str,
        top_k: usize,
        filters: Option<&SearchFilter>,
    ) -> Result<Vec<SearchResult>, AppError> {
        let index = match self.collections.read().get(collection).cloned() {
            Some(index) => index,
            None => return Ok(Vec::new()),
        };
        if collection_excluded_by_filter(collection, filters) {
            return Ok(Vec::new());
        }
        let terms = ngram_query_terms(index.field_search_text, query_text);
        if terms.is_empty() {
            return Ok(Vec::new());
        }
        let Some(query) = build_search_query(&index, terms, filters) else {
            return Ok(Vec::new());
        };
        tokio::task::spawn_blocking(move || -> Result<Vec<SearchResult>, AppError> {
            let searcher = index.reader.searcher();
            let hits = searcher
                .search(query.as_ref(), &TopDocs::with_limit(top_k))
                .map_err(|error| AppError::VectorStoreError(format!("BM25 检索失败: {error}")))?;
            let mut results = Vec::with_capacity(hits.len());
            for (score, address) in hits {
                let document: TantivyDocument = searcher.doc(address).map_err(|error| {
                    AppError::VectorStoreError(format!("读取文档失败: {error}"))
                })?;
                let id = required_text_field(&document, index.field_id, "id")?.to_string();
                if id.is_empty() {
                    return Err(AppError::VectorStoreError("BM25 结果包含空 id".to_string()));
                }
                let content =
                    required_text_field(&document, index.field_content, "content")?.to_string();
                let project_id = required_i64_field(&document, index.field_project, "project_id")?;
                let mut metadata = serde_json::json!({"project_id": project_id});
                if let serde_json::Value::Object(object) = &mut metadata {
                    if let Some(data_type) = optional_text_field(&document, index.field_data_type) {
                        object.insert("data_type".to_string(), serde_json::json!(data_type));
                    }
                    if let Some(chapter_number) =
                        optional_i64_field(&document, index.field_chapter_number)
                    {
                        object.insert(
                            "chapter_number".to_string(),
                            serde_json::json!(chapter_number),
                        );
                    }
                }
                results.push(SearchResult {
                    id,
                    content,
                    score,
                    vector_score: None,
                    bm25_score: Some(score),
                    metadata,
                });
            }
            results.sort_by(|left, right| right.score.total_cmp(&left.score));
            results.truncate(top_k);
            Ok(results)
        })
        .await
        .map_err(|error| AppError::VectorStoreError(format!("检索 task 失败: {error}")))?
    }
}
