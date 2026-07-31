//! `QdrantVectorStore`：完整 `VectorStore` 实现。

use super::config::QdrantStoreConfig;
use super::filters::{
    self, KEY_CHAPTER, KEY_CHUNK_ID, KEY_DATA_TYPE, KEY_ID_PREFIX, KEY_LOGICAL_COLLECTION,
    KEY_PROJECT_ID,
};
use crate::point_id::chunk_id_to_point_uuid;
use crate::sparse_text::{bm25_document, bm25_query};
use a4n_core::error::AppError;
use a4n_rag_core::vector_store::{
    CompletenessReport, DeleteFilter, EmbeddingRecord, SearchFilter, SearchResult, VectorStore,
};
use async_trait::async_trait;
use qdrant_client::Payload;
use qdrant_client::Qdrant;
use qdrant_client::qdrant::{
    CountPointsBuilder, CreateCollectionBuilder, CreateFieldIndexCollectionBuilder, Datatype,
    DeletePointsBuilder, Distance, FieldType, Filter, Fusion, GetPointsBuilder, Modifier,
    NamedVectors, PointId, PointStruct, PrefetchQueryBuilder, Query, QueryPointsBuilder,
    RetrievedPoint, ScoredPoint, ScrollPointsBuilder, SetPayloadPointsBuilder,
    SparseIndexConfigBuilder, SparseVectorParamsBuilder, SparseVectorsConfigBuilder,
    UpsertPointsBuilder, Vector, VectorInput, VectorParamsBuilder, VectorsConfigBuilder,
};
use serde_json::{Value, json};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use uuid::Uuid;

const DENSE_NAME: &str = "dense";
const SPARSE_NAME: &str = "bm25";

/// Qdrant 向量 + sparse BM25 存储。
pub struct QdrantVectorStore {
    client: Qdrant,
    base_collection: String,
}

impl QdrantVectorStore {
    pub async fn connect_from_env() -> Result<Arc<Self>, AppError> {
        let config = QdrantStoreConfig::from_env()?;
        Self::connect(config).await
    }

    pub async fn connect(config: QdrantStoreConfig) -> Result<Arc<Self>, AppError> {
        let mut builder = Qdrant::from_url(&config.url);
        if let Some(key) = &config.api_key {
            builder = builder.api_key(key.clone());
        }
        let client = builder
            .build()
            .map_err(|e| AppError::ConfigError(format!("连接 Qdrant 失败: {e}")))?;
        let store = Arc::new(Self {
            client,
            base_collection: config.collection,
        });
        store.client.list_collections().await.map_err(|e| {
            AppError::VectorStoreError(format!("Qdrant list_collections 失败: {e}"))
        })?;
        Ok(store)
    }

    fn map_err(context: &str, err: impl std::fmt::Display) -> AppError {
        AppError::VectorStoreError(format!("{context}: {err}"))
    }

    fn physical_collection(&self, logical_collection: &str) -> String {
        let id = Uuid::new_v5(&Uuid::NAMESPACE_OID, logical_collection.as_bytes());
        format!("{}__{}", self.base_collection, id.simple())
    }

    async fn existing_collection(
        &self,
        logical_collection: &str,
    ) -> Result<Option<String>, AppError> {
        let physical = self.physical_collection(logical_collection);
        self.client
            .collection_exists(&physical)
            .await
            .map(|exists| exists.then_some(physical))
            .map_err(|e| Self::map_err("collection_exists", e))
    }

    fn point_id(chunk_id: &str) -> PointId {
        PointId::from(chunk_id_to_point_uuid(chunk_id))
    }

    fn extract_project_id(metadata: &Value) -> Option<i64> {
        metadata.get("project_id").and_then(|v| v.as_i64())
    }

    fn extract_data_type(metadata: &Value) -> Option<String> {
        metadata
            .get("data_type")
            .and_then(|v| v.as_str())
            .map(str::to_string)
    }

    fn extract_chapter(metadata: &Value) -> Option<i64> {
        metadata
            .get("chapter_number")
            .and_then(|v| v.as_i64().or_else(|| v.as_u64().map(|u| u as i64)))
    }

    fn id_prefixes_for_chunk(chunk_id: &str) -> Vec<String> {
        let parts: Vec<&str> = chunk_id.split(':').collect();
        let mut out = Vec::new();
        if parts.len() >= 2 {
            for i in 1..parts.len() {
                out.push(format!("{}:", parts[..i].join(":")));
            }
        }
        out
    }

    async fn ensure_collection(
        &self,
        logical_collection: &str,
        dense_dim: usize,
    ) -> Result<String, AppError> {
        let physical = self.physical_collection(logical_collection);
        let exists = self
            .client
            .collection_exists(&physical)
            .await
            .map_err(|e| Self::map_err("collection_exists", e))?;
        if exists {
            return Ok(physical);
        }

        let mut vectors = VectorsConfigBuilder::default();
        vectors.add_named_vector_params(
            DENSE_NAME,
            VectorParamsBuilder::new(dense_dim as u64, Distance::Cosine),
        );
        let mut sparse = SparseVectorsConfigBuilder::default();
        sparse.add_named_vector_params(
            SPARSE_NAME,
            SparseVectorParamsBuilder::default()
                .modifier(Modifier::Idf)
                .index(SparseIndexConfigBuilder::default().datatype(Datatype::Float32)),
        );
        self.client
            .create_collection(
                CreateCollectionBuilder::new(&physical)
                    .vectors_config(vectors)
                    .sparse_vectors_config(sparse),
            )
            .await
            .map_err(|e| Self::map_err("create_collection", e))?;

        for (field, ty) in [
            (KEY_PROJECT_ID, FieldType::Integer),
            (KEY_CHAPTER, FieldType::Integer),
            (KEY_DATA_TYPE, FieldType::Keyword),
            (KEY_CHUNK_ID, FieldType::Keyword),
            (KEY_ID_PREFIX, FieldType::Keyword),
        ] {
            let _ = self
                .client
                .create_field_index(
                    CreateFieldIndexCollectionBuilder::new(&physical, field, ty).wait(true),
                )
                .await;
        }
        Ok(physical)
    }

    fn record_to_point(
        &self,
        logical_collection: &str,
        record: &EmbeddingRecord,
    ) -> Result<PointStruct, AppError> {
        if record.embedding.is_empty() {
            return Err(AppError::VectorStoreError(format!(
                "空向量: id={}",
                record.id
            )));
        }
        let mut named = NamedVectors::default().add_vector(DENSE_NAME, record.embedding.clone());
        let has_sparse =
            if let Some(content) = record.content.as_ref().filter(|c| !c.trim().is_empty()) {
                let sparse = bm25_document(content);
                if !sparse.is_empty() {
                    named = named.add_vector(
                        SPARSE_NAME,
                        Vector::new_sparse(sparse.indices, sparse.values),
                    );
                    true
                } else {
                    false
                }
            } else {
                false
            };

        let data_type = Self::extract_data_type(&record.metadata);
        let chapter = Self::extract_chapter(&record.metadata);
        let prefixes = Self::id_prefixes_for_chunk(&record.id);
        let project_id =
            Self::extract_project_id(&record.metadata).or_else(|| logical_collection.parse().ok());
        let mut payload_json = json!({
            KEY_CHUNK_ID: record.id,
            filters::KEY_LOGICAL_COLLECTION: logical_collection,
            "content": record.content,
            "metadata": record.metadata,
            "has_dense": true,
            "has_sparse": has_sparse,
            KEY_ID_PREFIX: prefixes,
        });
        if let Some(pid) = project_id {
            payload_json[KEY_PROJECT_ID] = json!(pid);
        }
        if let Some(dt) = data_type {
            payload_json[KEY_DATA_TYPE] = json!(dt);
        }
        if let Some(ch) = chapter {
            payload_json[KEY_CHAPTER] = json!(ch);
        }
        let payload = Payload::try_from(payload_json)
            .map_err(|e| AppError::VectorStoreError(format!("构造 payload 失败: {e}")))?;
        Ok(PointStruct::new(Self::point_id(&record.id), named, payload))
    }

    fn scored_to_result(point: &ScoredPoint) -> Result<SearchResult, AppError> {
        let chunk_id = point
            .payload
            .get(KEY_CHUNK_ID)
            .and_then(|v| v.as_str())
            .cloned()
            .unwrap_or_default();
        let content = point
            .payload
            .get("content")
            .and_then(|v| v.as_str())
            .cloned()
            .unwrap_or_default();
        let metadata = point
            .payload
            .get("metadata")
            .cloned()
            .map(Value::from)
            .unwrap_or(Value::Null);
        Ok(SearchResult {
            id: chunk_id,
            content,
            score: point.score,
            vector_score: Some(point.score),
            bm25_score: None,
            metadata,
        })
    }

    fn retrieved_meta(point: &RetrievedPoint) -> (String, Value, bool, bool) {
        let chunk_id = point
            .payload
            .get(KEY_CHUNK_ID)
            .and_then(|v| v.as_str())
            .cloned()
            .unwrap_or_default();
        let metadata = point
            .payload
            .get("metadata")
            .cloned()
            .map(Value::from)
            .unwrap_or(Value::Null);
        let has_dense = point
            .payload
            .get("has_dense")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);
        let has_sparse = point
            .payload
            .get("has_sparse")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        (chunk_id, metadata, has_dense, has_sparse)
    }

    async fn count_filter(&self, physical: &str, filter: Filter) -> Result<usize, AppError> {
        let resp = self
            .client
            .count(CountPointsBuilder::new(physical).filter(filter).exact(true))
            .await
            .map_err(|e| Self::map_err("count", e))?;
        Ok(resp.result.map(|r| r.count as usize).unwrap_or(0))
    }

    async fn delete_by_filter(&self, physical: &str, filter: Filter) -> Result<usize, AppError> {
        let before = self.count_filter(physical, filter.clone()).await?;
        if before == 0 {
            return Ok(0);
        }
        self.client
            .delete_points(DeletePointsBuilder::new(physical).points(filter).wait(true))
            .await
            .map_err(|e| Self::map_err("delete_points", e))?;
        Ok(before)
    }
}

#[async_trait]
impl VectorStore for QdrantVectorStore {
    async fn add_embeddings(
        &self,
        collection: &str,
        records: Vec<EmbeddingRecord>,
    ) -> Result<usize, AppError> {
        if records.is_empty() {
            return Ok(0);
        }
        let dim = records[0].embedding.len();
        for r in &records {
            if r.embedding.len() != dim {
                return Err(AppError::VectorStoreError(format!(
                    "批量写入维度不一致: id={}, expected={dim}, actual={}",
                    r.id,
                    r.embedding.len()
                )));
            }
        }
        let physical = self.ensure_collection(collection, dim).await?;
        let points: Result<Vec<_>, _> = records
            .iter()
            .map(|r| self.record_to_point(collection, r))
            .collect();
        let points = points?;
        let n = points.len();
        self.client
            .upsert_points(UpsertPointsBuilder::new(&physical, points).wait(true))
            .await
            .map_err(|e| Self::map_err("upsert_points", e))?;
        Ok(n)
    }

    async fn query(
        &self,
        collection: &str,
        query_embedding: Vec<f32>,
        top_k: usize,
        search_filters: Option<SearchFilter>,
    ) -> Result<Vec<SearchResult>, AppError> {
        if top_k == 0 {
            return Ok(Vec::new());
        }
        let Some(physical) = self.existing_collection(collection).await? else {
            return Ok(Vec::new());
        };
        let filter = filters::search_filter(collection, search_filters.as_ref());
        let resp = self
            .client
            .query(
                QueryPointsBuilder::new(&physical)
                    .query(Query::new_nearest(query_embedding))
                    .using(DENSE_NAME)
                    .filter(filter)
                    .limit(top_k as u64)
                    .with_payload(true),
            )
            .await
            .map_err(|e| Self::map_err("query dense", e))?;
        resp.result.iter().map(Self::scored_to_result).collect()
    }

    async fn search_text(
        &self,
        collection: &str,
        query_text: &str,
        top_k: usize,
        search_filters: Option<SearchFilter>,
    ) -> Result<Vec<SearchResult>, AppError> {
        if top_k == 0 {
            return Ok(Vec::new());
        }
        let sparse = bm25_query(query_text);
        if sparse.is_empty() {
            return Ok(Vec::new());
        }
        let Some(physical) = self.existing_collection(collection).await? else {
            return Ok(Vec::new());
        };
        let filter = filters::search_filter(collection, search_filters.as_ref());
        let resp = self
            .client
            .query(
                QueryPointsBuilder::new(&physical)
                    .query(VectorInput::new_sparse(sparse.indices, sparse.values))
                    .using(SPARSE_NAME)
                    .filter(filter)
                    .limit(top_k as u64)
                    .with_payload(true),
            )
            .await
            .map_err(|e| Self::map_err("query sparse", e))?;
        let mut results: Vec<SearchResult> = resp
            .result
            .iter()
            .map(Self::scored_to_result)
            .collect::<Result<_, _>>()?;
        for r in &mut results {
            r.bm25_score = r.vector_score.take();
        }
        Ok(results)
    }

    async fn hybrid_search(
        &self,
        collection: &str,
        query_text: &str,
        query_embedding: Vec<f32>,
        top_k: usize,
        search_filters: Option<SearchFilter>,
        _vector_weight: Option<f32>,
    ) -> Result<Vec<SearchResult>, AppError> {
        if top_k == 0 {
            return Ok(Vec::new());
        }
        let Some(physical) = self.existing_collection(collection).await? else {
            return Ok(Vec::new());
        };
        let pull = top_k.saturating_mul(2).max(1) as u64;
        let filter = filters::search_filter(collection, search_filters.as_ref());
        let sparse = bm25_query(query_text);

        let mut builder = QueryPointsBuilder::new(&physical)
            .limit(top_k as u64)
            .with_payload(true)
            .query(Query::new_fusion(Fusion::Rrf))
            .add_prefetch(
                PrefetchQueryBuilder::default()
                    .query(Query::new_nearest(query_embedding))
                    .using(DENSE_NAME)
                    .filter(filter.clone())
                    .limit(pull),
            );
        if !sparse.is_empty() {
            builder = builder.add_prefetch(
                PrefetchQueryBuilder::default()
                    .query(VectorInput::new_sparse(sparse.indices, sparse.values))
                    .using(SPARSE_NAME)
                    .filter(filter)
                    .limit(pull),
            );
        }
        let resp = self
            .client
            .query(builder)
            .await
            .map_err(|e| Self::map_err("hybrid query", e))?;
        resp.result.iter().map(Self::scored_to_result).collect()
    }

    async fn delete(&self, collection: &str, filter: DeleteFilter) -> Result<usize, AppError> {
        let Some(physical) = self.existing_collection(collection).await? else {
            return Ok(0);
        };
        let base = filters::delete_filter_base(collection, &filter);
        let qfilter = if !filter.ids.is_empty() {
            filters::with_chunk_ids(base, &filter.ids)
        } else if !filter.id_prefixes.is_empty() {
            filters::with_id_prefixes(base, &filter.id_prefixes)
        } else {
            base
        };
        self.delete_by_filter(&physical, qfilter).await
    }

    async fn collection_exists(&self, collection: &str) -> Result<bool, AppError> {
        Ok(self.existing_collection(collection).await?.is_some())
    }

    async fn count(&self, collection: &str) -> Result<usize, AppError> {
        let Some(physical) = self.existing_collection(collection).await? else {
            return Ok(0);
        };
        self.count_filter(&physical, filters::collection_filter(collection))
            .await
    }

    async fn drop_collection(&self, collection: &str) -> Result<(), AppError> {
        let Some(physical) = self.existing_collection(collection).await? else {
            return Ok(());
        };
        self.client
            .delete_collection(physical)
            .await
            .map_err(|e| Self::map_err("delete_collection", e))?;
        Ok(())
    }

    async fn list_collections(&self) -> Result<Vec<String>, AppError> {
        let prefix = format!("{}__", self.base_collection);
        let collections = self
            .client
            .list_collections()
            .await
            .map_err(|e| Self::map_err("list_collections", e))?;
        let mut projects = HashSet::new();
        for collection in collections
            .collections
            .into_iter()
            .filter(|collection| collection.name.starts_with(&prefix))
        {
            let resp = self
                .client
                .scroll(
                    ScrollPointsBuilder::new(&collection.name)
                        .limit(1)
                        .with_payload(true)
                        .with_vectors(false),
                )
                .await
                .map_err(|e| Self::map_err("scroll", e))?;
            if let Some(name) = resp.result.first().and_then(|point| {
                point
                    .payload
                    .get(KEY_LOGICAL_COLLECTION)
                    .and_then(|v| v.as_str())
                    .cloned()
            }) {
                projects.insert(name);
            }
        }
        let mut list: Vec<String> = projects.into_iter().collect();
        list.sort();
        Ok(list)
    }

    async fn update_metadata(
        &self,
        collection: &str,
        id: &str,
        metadata: Value,
    ) -> Result<(), AppError> {
        let mut payload_json = json!({ "metadata": metadata });
        if let Some(dt) = Self::extract_data_type(&payload_json["metadata"]) {
            payload_json[KEY_DATA_TYPE] = json!(dt);
        }
        if let Some(ch) = Self::extract_chapter(&payload_json["metadata"]) {
            payload_json[KEY_CHAPTER] = json!(ch);
        }
        let payload = Payload::try_from(payload_json)
            .map_err(|e| AppError::VectorStoreError(format!("payload: {e}")))?;
        let physical = self.physical_collection(collection);
        self.client
            .set_payload(
                SetPayloadPointsBuilder::new(&physical, payload)
                    .points_selector(vec![Self::point_id(id)])
                    .wait(true),
            )
            .await
            .map_err(|e| Self::map_err("set_payload", e))?;
        Ok(())
    }

    async fn metadata_by_ids(
        &self,
        collection: &str,
        ids: &[String],
    ) -> Result<HashMap<String, Value>, AppError> {
        if ids.is_empty() {
            return Ok(HashMap::new());
        }
        let Some(physical) = self.existing_collection(collection).await? else {
            return Ok(HashMap::new());
        };
        let point_ids: Vec<PointId> = ids.iter().map(|id| Self::point_id(id)).collect();
        let resp = self
            .client
            .get_points(
                GetPointsBuilder::new(&physical, point_ids)
                    .with_payload(true)
                    .with_vectors(false),
            )
            .await
            .map_err(|e| Self::map_err("get_points", e))?;
        let mut out = HashMap::new();
        for point in resp.result {
            let (chunk_id, metadata, _, _) = Self::retrieved_meta(&point);
            if !chunk_id.is_empty() {
                out.insert(chunk_id, metadata);
            }
        }
        Ok(out)
    }

    async fn metadata_by_filter(
        &self,
        collection: &str,
        filter: DeleteFilter,
    ) -> Result<HashMap<String, Value>, AppError> {
        let Some(physical) = self.existing_collection(collection).await? else {
            return Ok(HashMap::new());
        };
        let mut base = filters::delete_filter_base(collection, &filter);
        if !filter.ids.is_empty() {
            base = filters::with_chunk_ids(base, &filter.ids);
        } else if !filter.id_prefixes.is_empty() {
            base = filters::with_id_prefixes(base, &filter.id_prefixes);
        }
        let mut out = HashMap::new();
        let mut offset: Option<PointId> = None;
        loop {
            let mut builder = ScrollPointsBuilder::new(&physical)
                .filter(base.clone())
                .limit(256)
                .with_payload(true)
                .with_vectors(false);
            if let Some(off) = offset.clone() {
                builder = builder.offset(off);
            }
            let resp = self
                .client
                .scroll(builder)
                .await
                .map_err(|e| Self::map_err("scroll metadata", e))?;
            if resp.result.is_empty() {
                break;
            }
            for point in &resp.result {
                let (chunk_id, metadata, _, _) = Self::retrieved_meta(point);
                if !chunk_id.is_empty() {
                    out.insert(chunk_id, metadata);
                }
                offset = point.id.clone();
            }
            if resp.result.len() < 256 {
                break;
            }
        }
        Ok(out)
    }

    async fn health_check(&self) -> Result<bool, AppError> {
        self.client
            .list_collections()
            .await
            .map(|_| true)
            .map_err(|e| Self::map_err("health_check", e))
    }

    async fn completeness_report(
        &self,
        collection: &str,
        expected_count: usize,
    ) -> Result<CompletenessReport, AppError> {
        let actual = match self.existing_collection(collection).await? {
            Some(physical) => {
                self.count_filter(&physical, filters::collection_filter(collection))
                    .await?
            }
            None => 0,
        };
        Ok(CompletenessReport::count_only(
            collection,
            expected_count,
            actual,
            actual,
            Vec::new(),
            Vec::new(),
        ))
    }

    async fn completeness_report_by_ids(
        &self,
        collection: &str,
        expected_ids: &[String],
    ) -> Result<CompletenessReport, AppError> {
        if expected_ids.is_empty() {
            return Ok(CompletenessReport::by_expected_ids(
                collection,
                expected_ids,
                &HashSet::new(),
                Some(&HashSet::new()),
            ));
        }
        let Some(physical) = self.existing_collection(collection).await? else {
            return Ok(CompletenessReport::by_expected_ids(
                collection,
                expected_ids,
                &HashSet::new(),
                Some(&HashSet::new()),
            ));
        };
        let point_ids: Vec<PointId> = expected_ids.iter().map(|id| Self::point_id(id)).collect();
        let resp = self
            .client
            .get_points(
                GetPointsBuilder::new(&physical, point_ids)
                    .with_payload(true)
                    .with_vectors(false),
            )
            .await
            .map_err(|e| Self::map_err("get_points completeness", e))?;

        let mut vector_ids = HashSet::new();
        let mut bm25_ids = HashSet::new();
        for point in resp.result {
            let (chunk_id, _, has_dense, has_sparse) = Self::retrieved_meta(&point);
            if chunk_id.is_empty() {
                continue;
            }
            if has_dense {
                vector_ids.insert(chunk_id.clone());
            }
            if has_sparse {
                bm25_ids.insert(chunk_id);
            }
        }
        Ok(CompletenessReport::by_expected_ids(
            collection,
            expected_ids,
            &vector_ids,
            Some(&bm25_ids),
        ))
    }
}
