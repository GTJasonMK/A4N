//! `QdrantVectorStore`：完整 `VectorStore` 实现。

use super::config::QdrantStoreConfig;
use super::filters::{
    self, KEY_CHAPTER, KEY_CHUNK_ID, KEY_DATA_TYPE, KEY_ID_PREFIX,
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
    CountPointsBuilder, CountResult, CreateCollectionBuilder, CreateFieldIndexCollectionBuilder,
    Datatype, DeletePointsBuilder, Distance, FieldType, Filter, Fusion, GetPointsBuilder, Modifier,
    NamedVectors, PointId, PointStruct, PrefetchQueryBuilder, Query, QueryPointsBuilder,
    RetrievedPoint, ScoredPoint, ScrollPointsBuilder, SparseIndexConfigBuilder,
    SparseVectorParamsBuilder, SparseVectorsConfigBuilder, UpsertPointsBuilder,
    Value as QdrantValue, Vector, VectorInput, VectorParamsBuilder, VectorsConfigBuilder,
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
            (KEY_CHAPTER, FieldType::Integer),
            (KEY_DATA_TYPE, FieldType::Keyword),
            (KEY_CHUNK_ID, FieldType::Keyword),
            (KEY_ID_PREFIX, FieldType::Keyword),
        ] {
            if let Err(error) = self
                .client
                .create_field_index(
                    CreateFieldIndexCollectionBuilder::new(&physical, field, ty).wait(true),
                )
                .await
            {
                let error = format!("create_field_index `{field}` 失败: {error}");
                return match self.client.delete_collection(physical.clone()).await {
                    Ok(_) => Err(AppError::VectorStoreError(error)),
                    Err(cleanup_error) => Err(AppError::VectorStoreError(format!(
                        "{error}；删除未完成 Qdrant collection `{physical}` 失败: {cleanup_error}"
                    ))),
                };
            }
        }
        Ok(physical)
    }

    fn record_to_point(record: &EmbeddingRecord) -> Result<PointStruct, AppError> {
        if record.embedding.is_empty() {
            return Err(AppError::VectorStoreError(format!(
                "空向量: id={}",
                record.id
            )));
        }
        if record.content.trim().is_empty() {
            return Err(AppError::VectorStoreError(format!(
                "空内容: id={}",
                record.id
            )));
        }
        let mut named = NamedVectors::default().add_vector(DENSE_NAME, record.embedding.clone());
        let sparse = bm25_document(&record.content);
        let has_sparse = !sparse.is_empty();
        if has_sparse {
            named = named.add_vector(
                SPARSE_NAME,
                Vector::new_sparse(sparse.indices, sparse.values),
            );
        }

        let data_type = Self::extract_data_type(&record.metadata);
        let chapter = Self::extract_chapter(&record.metadata);
        let prefixes = Self::id_prefixes_for_chunk(&record.id);
        let mut payload_json = json!({
            KEY_CHUNK_ID: record.id,
            "content": record.content,
            "metadata": record.metadata,
            "has_dense": true,
            "has_sparse": has_sparse,
            KEY_ID_PREFIX: prefixes,
        });
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

    fn required_payload_string(
        payload: &HashMap<String, QdrantValue>,
        key: &str,
    ) -> Result<String, AppError> {
        payload
            .get(key)
            .and_then(|value| value.as_str())
            .filter(|value| !value.trim().is_empty())
            .cloned()
            .ok_or_else(|| {
                AppError::VectorStoreError(format!("Qdrant payload 缺少非空字符串字段 `{key}`"))
            })
    }

    fn required_payload_json(
        payload: &HashMap<String, QdrantValue>,
        key: &str,
    ) -> Result<Value, AppError> {
        let value = payload.get(key).cloned().map(Value::from).ok_or_else(|| {
            AppError::VectorStoreError(format!("Qdrant payload 缺少字段 `{key}`"))
        })?;
        if value.is_null() {
            return Err(AppError::VectorStoreError(format!(
                "Qdrant payload 字段 `{key}` 不能为 null"
            )));
        }
        Ok(value)
    }

    fn required_payload_bool(
        payload: &HashMap<String, QdrantValue>,
        key: &str,
    ) -> Result<bool, AppError> {
        payload
            .get(key)
            .and_then(|value| value.as_bool())
            .ok_or_else(|| {
                AppError::VectorStoreError(format!("Qdrant payload 缺少布尔字段 `{key}`"))
            })
    }

    fn scored_to_result(point: &ScoredPoint) -> Result<SearchResult, AppError> {
        let chunk_id = Self::required_payload_string(&point.payload, KEY_CHUNK_ID)?;
        let content = Self::required_payload_string(&point.payload, "content")?;
        let metadata = Self::required_payload_json(&point.payload, "metadata")?;
        Ok(SearchResult {
            id: chunk_id,
            content,
            score: point.score,
            metadata,
        })
    }

    fn retrieved_meta(point: &RetrievedPoint) -> Result<(String, Value, bool, bool), AppError> {
        Ok((
            Self::required_payload_string(&point.payload, KEY_CHUNK_ID)?,
            Self::required_payload_json(&point.payload, "metadata")?,
            Self::required_payload_bool(&point.payload, "has_dense")?,
            Self::required_payload_bool(&point.payload, "has_sparse")?,
        ))
    }

    fn required_count(result: Option<CountResult>) -> Result<usize, AppError> {
        let count = result
            .ok_or_else(|| {
                AppError::VectorStoreError("Qdrant count 成功响应缺少 result".to_string())
            })?
            .count;
        usize::try_from(count).map_err(|_| {
            AppError::VectorStoreError(format!("Qdrant count 超出 usize 范围: {count}"))
        })
    }

    async fn count_filter(&self, physical: &str, filter: Filter) -> Result<usize, AppError> {
        let resp = self
            .client
            .count(CountPointsBuilder::new(physical).filter(filter).exact(true))
            .await
            .map_err(|e| Self::map_err("count", e))?;
        Self::required_count(resp.result)
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
        let points: Result<Vec<_>, _> = records
            .iter()
            .map(Self::record_to_point)
            .collect();
        let points = points?;
        let physical = self.ensure_collection(collection, dim).await?;
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
        let filter = filters::search_filter(search_filters.as_ref())?;
        if top_k == 0 {
            return Ok(Vec::new());
        }
        let Some(physical) = self.existing_collection(collection).await? else {
            return Ok(Vec::new());
        };
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
        let filter = filters::search_filter(search_filters.as_ref())?;
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
        resp
            .result
            .iter()
            .map(Self::scored_to_result)
            .collect()
    }

    async fn hybrid_search(
        &self,
        collection: &str,
        query_text: &str,
        query_embedding: Vec<f32>,
        top_k: usize,
        search_filters: Option<SearchFilter>,
    ) -> Result<Vec<SearchResult>, AppError> {
        let filter = filters::search_filter(search_filters.as_ref())?;
        if top_k == 0 {
            return Ok(Vec::new());
        }
        let Some(physical) = self.existing_collection(collection).await? else {
            return Ok(Vec::new());
        };
        let pull = top_k.saturating_mul(2).max(1) as u64;
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
        let qfilter = filters::delete_filter(&filter.ids, &filter.id_prefixes);
        self.delete_by_filter(&physical, qfilter).await
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
            let (chunk_id, metadata, _, _) = Self::retrieved_meta(&point)?;
            out.insert(chunk_id, metadata);
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
        let base = filters::delete_filter(&filter.ids, &filter.id_prefixes);
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
                let (chunk_id, metadata, _, _) = Self::retrieved_meta(point)?;
                out.insert(chunk_id, metadata);
                offset = point.id.clone();
            }
            if resp.result.len() < 256 {
                break;
            }
        }
        Ok(out)
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
            let (chunk_id, _, has_dense, has_sparse) = Self::retrieved_meta(&point)?;
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

#[cfg(test)]
mod tests {
    use super::*;

    fn payload() -> HashMap<String, QdrantValue> {
        Payload::try_from(json!({
            KEY_CHUNK_ID: "chapter:7:1",
            "content": "章节正文",
            "metadata": { "project_id": 7 },
            "has_dense": true,
            "has_sparse": true,
        }))
        .expect("valid payload")
        .into()
    }

    #[test]
    fn qdrant_payload_decoding_rejects_missing_required_fields() {
        let result = QdrantVectorStore::scored_to_result(&ScoredPoint {
            payload: payload(),
            score: 0.75,
            ..Default::default()
        })
        .expect("valid search payload");
        assert_eq!(result.id, "chapter:7:1");
        assert_eq!(result.content, "章节正文");
        assert_eq!(result.metadata["project_id"], 7);

        let missing_chunk_id = QdrantVectorStore::scored_to_result(&ScoredPoint::default())
            .expect_err("missing payload must not become an empty search result");
        assert!(missing_chunk_id.to_string().contains("chunk_id"));

        let mut missing_sparse = payload();
        missing_sparse.remove("has_sparse");
        let missing_sparse = QdrantVectorStore::retrieved_meta(&RetrievedPoint {
            payload: missing_sparse,
            ..Default::default()
        })
        .expect_err("missing completeness flags must not use defaults");
        assert!(missing_sparse.to_string().contains("has_sparse"));
    }

    #[test]
    fn qdrant_count_response_requires_result() {
        assert_eq!(
            QdrantVectorStore::required_count(Some(CountResult { count: 3 }))
                .expect("count result"),
            3
        );
        let error = QdrantVectorStore::required_count(None)
            .expect_err("missing result must not become count zero");
        assert!(error.to_string().contains("缺少 result"));
    }
}
