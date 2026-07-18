use std::collections::HashMap;

use a4n_core::error::AppError;
use a4n_rag_core::vector_store::DeleteFilter;
use futures::TryStreamExt;
use lancedb::query::{ExecutableQuery, QueryBase, Select};

use super::connection::LanceVectorStore;
use super::decode::batches_to_metadata;
use super::sql::{delete_filter_sql, ids_in_clause};

impl LanceVectorStore {
    pub(super) async fn update_metadata_impl(
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
            .map_err(|err| AppError::VectorStoreError(format!("open_table 失败: {err}")))?;

        let json = serde_json::to_string(&metadata)
            .map_err(|err| AppError::VectorStoreError(format!("metadata 序列化失败: {err}")))?;
        let sql_literal = format!("'{}'", json.replace('\'', "''"));
        let escaped_id = id.replace('\'', "''");
        let filter = format!("id = '{escaped_id}'");

        table
            .update()
            .only_if(filter)
            .column("metadata", sql_literal)
            .execute()
            .await
            .map_err(|err| AppError::VectorStoreError(format!("update_metadata 失败: {err}")))?;
        Ok(())
    }

    pub(super) async fn metadata_by_ids_impl(
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
            .map_err(|err| AppError::VectorStoreError(format!("open_table 失败: {err}")))?;
        let stream = table
            .query()
            .only_if(ids_in_clause(ids))
            .select(Select::columns(&["id", "metadata"]))
            .limit(ids.len())
            .execute()
            .await
            .map_err(|err| {
                AppError::VectorStoreError(format!("metadata_by_ids 查询失败: {err}"))
            })?;
        collect_metadata(stream, "metadata_by_ids 收集失败").await
    }

    pub(super) async fn metadata_by_filter_impl(
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
            .map_err(|err| AppError::VectorStoreError(format!("open_table 失败: {err}")))?;
        let stream = table
            .query()
            .only_if(delete_filter_sql(&filter))
            .select(Select::columns(&["id", "metadata"]))
            .execute()
            .await
            .map_err(|err| {
                AppError::VectorStoreError(format!("metadata_by_filter 查询失败: {err}"))
            })?;
        collect_metadata(stream, "metadata_by_filter 收集失败").await
    }
}

async fn collect_metadata(
    mut stream: impl futures::Stream<Item = Result<arrow_array::RecordBatch, lancedb::Error>> + Unpin,
    error_context: &str,
) -> Result<HashMap<String, serde_json::Value>, AppError> {
    let mut batches = Vec::new();
    while let Some(batch) = stream
        .try_next()
        .await
        .map_err(|err| AppError::VectorStoreError(format!("{error_context}: {err}")))?
    {
        batches.push(batch);
    }
    batches_to_metadata(batches)
}
