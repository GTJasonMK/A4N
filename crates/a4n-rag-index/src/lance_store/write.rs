use a4n_core::error::AppError;
use a4n_rag_core::vector_store::EmbeddingRecord;

use super::batch::{build_schema, records_to_batch, validate_embedding_vector};
use super::connection::LanceVectorStore;
use super::sql::ids_in_clause;

impl LanceVectorStore {
    pub(super) async fn add_embeddings_impl(
        &self,
        collection: &str,
        records: Vec<EmbeddingRecord>,
    ) -> Result<usize, AppError> {
        if records.is_empty() {
            return Ok(0);
        }
        let table_name = Self::sanitize(collection);
        let dimension =
            validate_embedding_vector(&table_name, &records[0].id, &records[0].embedding)?;
        for record in &records {
            let actual = validate_embedding_vector(&table_name, &record.id, &record.embedding)?;
            if actual != dimension {
                return Err(AppError::VectorStoreError(format!(
                    "批量写入包含不同维度向量：collection={table_name}, id={}, expected_dimension={dimension}, actual_dimension={actual}",
                    record.id
                )));
            }
        }

        match self.cached_dim(&table_name) {
            Some(existing) if existing != dimension => {
                return Err(AppError::VectorStoreError(format!(
                    "collection {table_name} 既有维度 {existing}，与新记录维度 {dimension} 不匹配"
                )));
            }
            None => self.remember_dim(&table_name, dimension),
            _ => {}
        }

        let schema = build_schema(dimension)?;
        let batch = records_to_batch(schema.clone(), dimension, &records)?;
        let count = records.len();
        let ids: Vec<String> = records.iter().map(|record| record.id.clone()).collect();

        let names = self
            .db
            .table_names()
            .execute()
            .await
            .map_err(|err| AppError::VectorStoreError(format!("table_names 失败: {err}")))?;

        if names.iter().any(|name| name == &table_name) {
            let table = self
                .db
                .open_table(&table_name)
                .execute()
                .await
                .map_err(|err| AppError::VectorStoreError(format!("open_table 失败: {err}")))?;
            table.delete(&ids_in_clause(&ids)).await.map_err(|err| {
                AppError::VectorStoreError(format!("upsert 删除旧记录失败: {err}"))
            })?;
            table
                .add(batch)
                .execute()
                .await
                .map_err(|err| AppError::VectorStoreError(format!("add 失败: {err}")))?;
        } else {
            self.db
                .create_table(&table_name, batch)
                .execute()
                .await
                .map_err(|err| AppError::VectorStoreError(format!("create_table 失败: {err}")))?;
        }

        Ok(count)
    }
}
