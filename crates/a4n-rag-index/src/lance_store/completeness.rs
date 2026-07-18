use a4n_core::error::AppError;
use a4n_rag_core::vector_store::CompletenessReport;

use super::connection::LanceVectorStore;
use super::sql::escape_sql;

impl LanceVectorStore {
    pub(super) async fn completeness_report_impl(
        &self,
        collection: &str,
        expected_count: usize,
    ) -> Result<CompletenessReport, AppError> {
        let actual = self.count_impl(collection).await?;
        let missing = if actual < expected_count {
            (actual..expected_count)
                .map(|index| format!("missing-vector-{index}"))
                .collect()
        } else {
            Vec::new()
        };
        let actual = usize::try_from(actual).map_err(|_| {
            AppError::VectorStoreError(format!("collection 行数超出 usize 范围: {actual}"))
        })?;
        Ok(CompletenessReport::from_missing_ids(
            collection.to_string(),
            expected_count,
            actual,
            0,
            missing,
            Vec::new(),
        ))
    }

    pub(super) async fn completeness_report_by_ids_impl(
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
            .map_err(|err| AppError::VectorStoreError(format!("open_table 失败: {err}")))?;
        let actual = table
            .count_rows(None)
            .await
            .map_err(|err| AppError::VectorStoreError(format!("count_rows 失败: {err}")))?;

        let mut missing = Vec::new();
        for id in expected_ids {
            let filter = format!("id = '{}'", escape_sql(id));
            let count = table.count_rows(Some(filter)).await.map_err(|err| {
                AppError::VectorStoreError(format!("检查记录 id={id} 是否存在失败: {err}"))
            })?;
            if count == 0 {
                missing.push(id.clone());
            }
        }

        Ok(CompletenessReport::from_missing_ids(
            collection.to_string(),
            expected_ids.len(),
            actual,
            0,
            missing,
            Vec::new(),
        ))
    }
}
