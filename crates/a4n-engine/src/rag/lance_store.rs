//! LanceDB 向量存储后端
//!
//! 在 `data_dir/lance/` 下维护一个 LanceDB 数据库；每个 collection 对应一张 lance 表。
//! 提供 IVF/HNSW 等 ANN 加速能力，作为当前桌面端持久化向量存储实现。
//!
//! 设计要点：
//! - 维度按每个 collection 首条记录推断，schema 一旦建立不可变
//! - upsert 语义通过 "delete by id list + add" 组合实现
//! - SearchFilter 支持 project_id / data_type / chapter_range 三类常用过滤
//! - 元数据 JSON 整体序列化进 `metadata` 列（Utf8），保留原始结构

use std::collections::HashMap;
use std::path::PathBuf;

use a4n_core::error::AppError;
use async_trait::async_trait;
use futures::TryStreamExt;
use lancedb::{
    Connection, connect,
    query::{ExecutableQuery, QueryBase, Select},
};
use parking_lot::RwLock;

use super::vector_store::{
    CompletenessReport, DeleteFilter, EmbeddingRecord, SearchFilter, SearchResult, VectorStore,
};

mod batch;
mod decode;
mod sql;

#[cfg(test)]
mod tests;

use batch::{build_schema, dim_of_vector_field, records_to_batch, validate_embedding_vector};
use decode::{batches_to_metadata, batches_to_results};
use sql::{
    delete_filter_sql, escape_sql, ids_in_clause, search_filter_fully_expressed_by_sql,
    search_filter_sql,
};

/// LanceDB 向量存储
pub struct LanceVectorStore {
    db: Connection,
    /// 每个 collection 的向量维度缓存（首次写入时确定，后续校验）
    dims: RwLock<HashMap<String, usize>>,
}

impl LanceVectorStore {
    /// 打开或创建 LanceDB 数据库
    pub async fn open(root: PathBuf) -> Result<Self, AppError> {
        std::fs::create_dir_all(&root)
            .map_err(|e| AppError::VectorStoreError(format!("创建 lance 根目录失败: {e}")))?;
        let uri = root.to_string_lossy().to_string();
        let db = connect(&uri)
            .execute()
            .await
            .map_err(|e| AppError::VectorStoreError(format!("lancedb connect 失败: {e}")))?;

        let mut store = Self {
            db,
            dims: RwLock::new(HashMap::new()),
        };
        store.refresh_dims_cache().await?;
        Ok(store)
    }

    /// 从已有 collection 的 schema 读出维度填入缓存
    async fn refresh_dims_cache(&mut self) -> Result<(), AppError> {
        let names = self
            .db
            .table_names()
            .execute()
            .await
            .map_err(|e| AppError::VectorStoreError(format!("table_names 失败: {e}")))?;
        let mut refreshed_dims = HashMap::new();
        for name in names {
            let table =
                self.db.open_table(&name).execute().await.map_err(|e| {
                    AppError::VectorStoreError(format!("open_table {name} 失败: {e}"))
                })?;
            let schema = table
                .schema()
                .await
                .map_err(|e| AppError::VectorStoreError(format!("schema {name} 失败: {e}")))?;
            if let Some(d) = dim_of_vector_field(schema.as_ref()) {
                refreshed_dims.insert(name, d);
            }
        }
        *self.dims.write() = refreshed_dims;
        Ok(())
    }

    async fn table_exists(&self, table_name: &str) -> Result<bool, AppError> {
        let names = self
            .db
            .table_names()
            .execute()
            .await
            .map_err(|e| AppError::VectorStoreError(format!("table_names 失败: {e}")))?;
        Ok(names.iter().any(|n| n == table_name))
    }

    fn sanitize(name: &str) -> String {
        name.chars()
            .map(|c| {
                if c.is_alphanumeric() || c == '_' || c == '-' {
                    c
                } else {
                    '_'
                }
            })
            .collect()
    }

    fn cached_dim(&self, collection: &str) -> Option<usize> {
        self.dims.read().get(collection).copied()
    }

    fn remember_dim(&self, collection: &str, dim: usize) {
        self.dims.write().insert(collection.to_string(), dim);
    }

    fn forget_dim(&self, collection: &str) {
        self.dims.write().remove(collection);
    }
}

#[async_trait]
impl VectorStore for LanceVectorStore {
    async fn add_embeddings(
        &self,
        collection: &str,
        records: Vec<EmbeddingRecord>,
    ) -> Result<usize, AppError> {
        if records.is_empty() {
            return Ok(0);
        }
        let table_name = Self::sanitize(collection);
        let dim = validate_embedding_vector(&table_name, &records[0].id, &records[0].embedding)?;
        for record in &records {
            let actual = validate_embedding_vector(&table_name, &record.id, &record.embedding)?;
            if actual != dim {
                return Err(AppError::VectorStoreError(format!(
                    "批量写入包含不同维度向量：collection={table_name}, id={}, expected_dimension={dim}, actual_dimension={actual}",
                    record.id
                )));
            }
        }

        // 校验 / 记忆维度
        match self.cached_dim(&table_name) {
            Some(existing) if existing != dim => {
                return Err(AppError::VectorStoreError(format!(
                    "collection {table_name} 既有维度 {existing}，与新记录维度 {dim} 不匹配"
                )));
            }
            None => self.remember_dim(&table_name, dim),
            _ => {}
        }

        let schema = build_schema(dim);
        let batch = records_to_batch(schema.clone(), dim, &records)?;
        let n = records.len();

        // upsert：先删除同 id 的旧行
        let id_list: Vec<String> = records.iter().map(|r| r.id.clone()).collect();

        let names = self
            .db
            .table_names()
            .execute()
            .await
            .map_err(|e| AppError::VectorStoreError(format!("table_names 失败: {e}")))?;

        if names.iter().any(|n| n == &table_name) {
            let table = self
                .db
                .open_table(&table_name)
                .execute()
                .await
                .map_err(|e| AppError::VectorStoreError(format!("open_table 失败: {e}")))?;
            // 删除同 id 旧记录
            table
                .delete(&ids_in_clause(&id_list))
                .await
                .map_err(|e| AppError::VectorStoreError(format!("upsert 删除旧记录失败: {e}")))?;
            table
                .add(batch)
                .execute()
                .await
                .map_err(|e| AppError::VectorStoreError(format!("add 失败: {e}")))?;
        } else {
            self.db
                .create_table(&table_name, batch)
                .execute()
                .await
                .map_err(|e| AppError::VectorStoreError(format!("create_table 失败: {e}")))?;
        }

        Ok(n)
    }

    async fn query(
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
        let query_dim = validate_embedding_vector(&table_name, "<query>", &query_embedding)?;
        match self.cached_dim(&table_name) {
            Some(existing) if existing != query_dim => {
                return Err(AppError::VectorStoreError(format!(
                    "查询向量维度与 collection 不一致：collection={table_name}, query_dimension={query_dim}, collection_dimension={existing}"
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
            .map_err(|e| AppError::VectorStoreError(format!("open_table 失败: {e}")))?;

        let needs_metadata_post_filter = filters
            .as_ref()
            .is_some_and(|filter| !search_filter_fully_expressed_by_sql(filter));
        let limit = if needs_metadata_post_filter {
            self.count(collection).await?
        } else {
            top_k
        };

        let mut query = table
            .query()
            .nearest_to(query_embedding)
            .map_err(|e| AppError::VectorStoreError(format!("nearest_to 失败: {e}")))?
            .limit(limit);
        if let Some(sql) = search_filter_sql(filters.as_ref()) {
            query = query.only_if(sql);
        }
        let stream = query
            .execute()
            .await
            .map_err(|e| AppError::VectorStoreError(format!("query execute 失败: {e}")))?;
        let mut batches = Vec::new();
        let mut s = stream;
        while let Some(batch) = s
            .try_next()
            .await
            .map_err(|e| AppError::VectorStoreError(format!("收集结果失败: {e}")))?
        {
            batches.push(batch);
        }

        let mut results = batches_to_results(batches)?;
        if let Some(filter) = filters.as_ref().filter(|_| needs_metadata_post_filter) {
            results.retain(|result| {
                super::vector_store::metadata_matches_search_filter(&result.metadata, filter)
            });
            results.truncate(top_k);
        }
        Ok(results)
    }

    async fn search_text(
        &self,
        _collection: &str,
        _query_text: &str,
        _top_k: usize,
        _filters: Option<SearchFilter>,
    ) -> Result<Vec<SearchResult>, AppError> {
        // lance 端不负责 BM25 全文搜索；由 HybridVectorStore 转给 TantivyTextIndex
        Ok(vec![])
    }

    async fn hybrid_search(
        &self,
        collection: &str,
        _query_text: &str,
        query_embedding: Vec<f32>,
        top_k: usize,
        filters: Option<SearchFilter>,
        _vector_weight: Option<f32>,
    ) -> Result<Vec<SearchResult>, AppError> {
        // lance 端单独走向量搜索；融合由 HybridVectorStore 控制
        self.query(collection, query_embedding, top_k, filters)
            .await
    }

    async fn delete(&self, collection: &str, filter: DeleteFilter) -> Result<usize, AppError> {
        let table_name = Self::sanitize(collection);
        if !self.table_exists(&table_name).await? {
            return Ok(0);
        }
        let table = self
            .db
            .open_table(&table_name)
            .execute()
            .await
            .map_err(|e| AppError::VectorStoreError(format!("open_table 失败: {e}")))?;
        // lance 的 delete 不返回删除行数，先 count 再 delete 估算
        let sql = delete_filter_sql(&filter);
        // 估算删除前条数（轻量）
        let before = table
            .count_rows(None)
            .await
            .map_err(|e| AppError::VectorStoreError(format!("delete 前 count_rows 失败: {e}")))?;
        table
            .delete(&sql)
            .await
            .map_err(|e| AppError::VectorStoreError(format!("delete 失败: {e}")))?;
        let after = table
            .count_rows(None)
            .await
            .map_err(|e| AppError::VectorStoreError(format!("delete 后 count_rows 失败: {e}")))?;
        Ok(before.saturating_sub(after) as usize)
    }

    async fn collection_exists(&self, collection: &str) -> Result<bool, AppError> {
        let table_name = Self::sanitize(collection);
        self.table_exists(&table_name).await
    }

    async fn create_collection(&self, _collection: &str) -> Result<(), AppError> {
        // 维度未知时无法预建表；交由首次 add_embeddings 创建
        Ok(())
    }

    async fn count(&self, collection: &str) -> Result<usize, AppError> {
        let table_name = Self::sanitize(collection);
        if !self.table_exists(&table_name).await? {
            return Ok(0);
        }
        let table = self
            .db
            .open_table(&table_name)
            .execute()
            .await
            .map_err(|e| AppError::VectorStoreError(format!("open_table 失败: {e}")))?;
        let n = table
            .count_rows(None)
            .await
            .map_err(|e| AppError::VectorStoreError(format!("count_rows 失败: {e}")))?;
        Ok(n as usize)
    }

    async fn drop_collection(&self, collection: &str) -> Result<(), AppError> {
        let table_name = Self::sanitize(collection);
        if !self.table_exists(&table_name).await? {
            self.forget_dim(&table_name);
            return Ok(());
        }
        self.db
            .drop_table(&table_name, &[] as &[String])
            .await
            .map_err(|e| AppError::VectorStoreError(format!("drop_table 失败: {e}")))?;
        self.forget_dim(&table_name);
        Ok(())
    }

    async fn list_collections(&self) -> Result<Vec<String>, AppError> {
        self.db
            .table_names()
            .execute()
            .await
            .map_err(|e| AppError::VectorStoreError(format!("table_names 失败: {e}")))
    }

    async fn update_metadata(
        &self,
        collection: &str,
        id: &str,
        metadata: serde_json::Value,
    ) -> Result<(), AppError> {
        let table_name = Self::sanitize(collection);
        let table = self
            .db
            .open_table(&table_name)
            .execute()
            .await
            .map_err(|e| AppError::VectorStoreError(format!("open_table 失败: {e}")))?;

        // metadata 列以 JSON 字符串存储；用 SQL literal 设置（注意 ' 转义）
        let json_str = serde_json::to_string(&metadata)
            .map_err(|e| AppError::VectorStoreError(format!("metadata 序列化失败: {e}")))?;
        let sql_literal = format!("'{}'", json_str.replace('\'', "''"));

        let escaped_id = id.replace('\'', "''");
        let filter = format!("id = '{escaped_id}'");

        table
            .update()
            .only_if(filter)
            .column("metadata", sql_literal)
            .execute()
            .await
            .map_err(|e| AppError::VectorStoreError(format!("update_metadata 失败: {e}")))?;
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
        let table_name = Self::sanitize(collection);
        if !self.table_exists(&table_name).await? {
            return Ok(HashMap::new());
        }
        let table = self
            .db
            .open_table(&table_name)
            .execute()
            .await
            .map_err(|e| AppError::VectorStoreError(format!("open_table 失败: {e}")))?;
        let stream = table
            .query()
            .only_if(ids_in_clause(ids))
            .select(Select::columns(&["id", "metadata"]))
            .limit(ids.len())
            .execute()
            .await
            .map_err(|e| AppError::VectorStoreError(format!("metadata_by_ids 查询失败: {e}")))?;
        let mut batches = Vec::new();
        let mut stream = stream;
        while let Some(batch) = stream
            .try_next()
            .await
            .map_err(|e| AppError::VectorStoreError(format!("metadata_by_ids 收集失败: {e}")))?
        {
            batches.push(batch);
        }
        batches_to_metadata(batches)
    }

    async fn metadata_by_filter(
        &self,
        collection: &str,
        filter: DeleteFilter,
    ) -> Result<HashMap<String, serde_json::Value>, AppError> {
        let table_name = Self::sanitize(collection);
        if !self.table_exists(&table_name).await? {
            return Ok(HashMap::new());
        }
        let table = self
            .db
            .open_table(&table_name)
            .execute()
            .await
            .map_err(|e| AppError::VectorStoreError(format!("open_table 失败: {e}")))?;
        let stream = table
            .query()
            .only_if(delete_filter_sql(&filter))
            .select(Select::columns(&["id", "metadata"]))
            .execute()
            .await
            .map_err(|e| AppError::VectorStoreError(format!("metadata_by_filter 查询失败: {e}")))?;
        let mut batches = Vec::new();
        let mut stream = stream;
        while let Some(batch) = stream
            .try_next()
            .await
            .map_err(|e| AppError::VectorStoreError(format!("metadata_by_filter 收集失败: {e}")))?
        {
            batches.push(batch);
        }
        batches_to_metadata(batches)
    }

    async fn health_check(&self) -> Result<bool, AppError> {
        // 能列表即视为健康
        self.db
            .table_names()
            .execute()
            .await
            .map(|_| true)
            .map_err(|e| AppError::VectorStoreError(format!("health_check 失败: {e}")))
    }

    async fn completeness_report(
        &self,
        collection: &str,
        expected_count: usize,
    ) -> Result<CompletenessReport, AppError> {
        let actual = self.count(collection).await?;
        let missing = if actual < expected_count {
            (actual..expected_count)
                .map(|i| format!("missing-vector-{i}"))
                .collect()
        } else {
            Vec::new()
        };
        Ok(CompletenessReport::from_missing_ids(
            collection.to_string(),
            expected_count,
            actual,
            0,
            missing,
            Vec::new(),
        ))
    }

    async fn completeness_report_by_ids(
        &self,
        collection: &str,
        expected_ids: &[String],
    ) -> Result<CompletenessReport, AppError> {
        let table_name = Self::sanitize(collection);
        if !self.table_exists(&table_name).await? {
            return Ok(CompletenessReport::from_missing_ids(
                collection.to_string(),
                expected_ids.len(),
                0,
                0,
                expected_ids.to_vec(),
                Vec::new(),
            ));
        }

        let table = self
            .db
            .open_table(&table_name)
            .execute()
            .await
            .map_err(|e| AppError::VectorStoreError(format!("open_table 失败: {e}")))?;
        let actual = table
            .count_rows(None)
            .await
            .map_err(|e| AppError::VectorStoreError(format!("count_rows 失败: {e}")))?;

        let mut missing = Vec::new();
        for id in expected_ids {
            let filter = format!("id = '{}'", escape_sql(id));
            let count = table.count_rows(Some(filter)).await.map_err(|e| {
                AppError::VectorStoreError(format!("检查记录 id={id} 是否存在失败: {e}"))
            })?;
            if count == 0 {
                missing.push(id.clone());
            }
        }

        Ok(CompletenessReport::from_missing_ids(
            collection.to_string(),
            expected_ids.len(),
            actual as usize,
            0,
            missing,
            Vec::new(),
        ))
    }
}
