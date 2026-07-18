use a4n_core::error::AppError;
use a4n_rag_core::vector_store::{SearchFilter, SearchResult};
use futures::TryStreamExt;
use lancedb::query::{ExecutableQuery, QueryBase};

use super::batch::validate_embedding_vector;
use super::connection::LanceVectorStore;
use super::decode::batches_to_results;
use super::sql::{search_filter_fully_expressed_by_sql, search_filter_sql};

impl LanceVectorStore {
    pub(super) async fn query_impl(
        &self,
        collection: &str,
        query_embedding: Vec<f32>,
        top_k: usize,
        filters: Option<SearchFilter>,
    ) -> Result<Vec<SearchResult>, AppError> {
        let table_name = Self::sanitize(collection);
        if !self.table_exists(&table_name).await? {
            return Ok(vec![]);
        }
        let query_dimension = validate_embedding_vector(&table_name, "<query>", &query_embedding)?;
        match self.cached_dim(&table_name) {
            Some(existing) if existing != query_dimension => {
                return Err(AppError::VectorStoreError(format!(
                    "查询向量维度与 collection 不一致：collection={table_name}, query_dimension={query_dimension}, collection_dimension={existing}"
                )));
            }
            Some(_) => {}
            None => {
                return Err(AppError::VectorStoreError(format!(
                    "collection {table_name} 缺少向量维度缓存"
                )));
            }
        }
        let table = self
            .db
            .open_table(&table_name)
            .execute()
            .await
            .map_err(|err| AppError::VectorStoreError(format!("open_table 失败: {err}")))?;

        let needs_metadata_post_filter = filters
            .as_ref()
            .is_some_and(|filter| !search_filter_fully_expressed_by_sql(filter));
        let limit = if needs_metadata_post_filter {
            self.count_impl(collection).await?
        } else {
            top_k
        };

        let mut query = table
            .query()
            .nearest_to(query_embedding)
            .map_err(|err| AppError::VectorStoreError(format!("nearest_to 失败: {err}")))?
            .limit(limit);
        if let Some(sql) = search_filter_sql(filters.as_ref()) {
            query = query.only_if(sql);
        }
        let stream = query
            .execute()
            .await
            .map_err(|err| AppError::VectorStoreError(format!("query execute 失败: {err}")))?;
        let mut batches = Vec::new();
        let mut stream = stream;
        while let Some(batch) = stream
            .try_next()
            .await
            .map_err(|err| AppError::VectorStoreError(format!("收集结果失败: {err}")))?
        {
            batches.push(batch);
        }

        let mut results = batches_to_results(batches)?;
        if let Some(filter) = filters.as_ref().filter(|_| needs_metadata_post_filter) {
            results.retain(|result| {
                a4n_rag_core::vector_store::metadata_matches_search_filter(&result.metadata, filter)
            });
            results.truncate(top_k);
        }
        Ok(results)
    }

    pub(super) async fn search_text_impl(
        &self,
        _collection: &str,
        _query_text: &str,
        _top_k: usize,
        _filters: Option<SearchFilter>,
    ) -> Result<Vec<SearchResult>, AppError> {
        Ok(vec![])
    }

    pub(super) async fn hybrid_search_impl(
        &self,
        collection: &str,
        _query_text: &str,
        query_embedding: Vec<f32>,
        top_k: usize,
        filters: Option<SearchFilter>,
        _vector_weight: Option<f32>,
    ) -> Result<Vec<SearchResult>, AppError> {
        self.query_impl(collection, query_embedding, top_k, filters)
            .await
    }
}
