use a4n_core::error::AppError;
use a4n_rag_core::vector_store::DeleteFilter;

use super::connection::LanceVectorStore;
use super::sql::delete_filter_sql;

impl LanceVectorStore {
    pub(super) async fn delete_impl(
        &self,
        collection: &str,
        filter: DeleteFilter,
    ) -> Result<usize, AppError> {
        let table_name = Self::sanitize(collection);
        if !self.table_exists(&table_name).await? {
            return Ok(0);
        }
        let table = self
            .db
            .open_table(&table_name)
            .execute()
            .await
            .map_err(|err| AppError::VectorStoreError(format!("open_table 失败: {err}")))?;
        let sql = delete_filter_sql(&filter);
        let before = table.count_rows(None).await.map_err(|err| {
            AppError::VectorStoreError(format!("delete 前 count_rows 失败: {err}"))
        })?;
        table
            .delete(&sql)
            .await
            .map_err(|err| AppError::VectorStoreError(format!("delete 失败: {err}")))?;
        let after = table.count_rows(None).await.map_err(|err| {
            AppError::VectorStoreError(format!("delete 后 count_rows 失败: {err}"))
        })?;
        let deleted = before.checked_sub(after).ok_or_else(|| {
            AppError::VectorStoreError(format!(
                "delete 后 collection 行数反而增加: before={before}, after={after}"
            ))
        })?;
        usize::try_from(deleted).map_err(|_| {
            AppError::VectorStoreError(format!("删除记录数量超出 usize 范围: {deleted}"))
        })
    }

    pub(super) async fn collection_exists_impl(&self, collection: &str) -> Result<bool, AppError> {
        let table_name = Self::sanitize(collection);
        self.table_exists(&table_name).await
    }

    pub(super) async fn count_impl(&self, collection: &str) -> Result<usize, AppError> {
        let table_name = Self::sanitize(collection);
        if !self.table_exists(&table_name).await? {
            return Ok(0);
        }
        let table = self
            .db
            .open_table(&table_name)
            .execute()
            .await
            .map_err(|err| AppError::VectorStoreError(format!("open_table 失败: {err}")))?;
        let count = table
            .count_rows(None)
            .await
            .map_err(|err| AppError::VectorStoreError(format!("count_rows 失败: {err}")))?;
        usize::try_from(count).map_err(|_| {
            AppError::VectorStoreError(format!("collection 行数超出 usize 范围: {count}"))
        })
    }

    pub(super) async fn drop_collection_impl(&self, collection: &str) -> Result<(), AppError> {
        let table_name = Self::sanitize(collection);
        if !self.table_exists(&table_name).await? {
            self.forget_dim(&table_name);
            return Ok(());
        }
        self.db
            .drop_table(&table_name, &[] as &[String])
            .await
            .map_err(|err| AppError::VectorStoreError(format!("drop_table 失败: {err}")))?;
        self.forget_dim(&table_name);
        Ok(())
    }

    pub(super) async fn list_collections_impl(&self) -> Result<Vec<String>, AppError> {
        self.db
            .table_names()
            .execute()
            .await
            .map_err(|err| AppError::VectorStoreError(format!("table_names 失败: {err}")))
    }

    pub(super) async fn health_check_impl(&self) -> Result<bool, AppError> {
        self.db
            .table_names()
            .execute()
            .await
            .map(|_| true)
            .map_err(|err| AppError::VectorStoreError(format!("health_check 失败: {err}")))
    }
}
