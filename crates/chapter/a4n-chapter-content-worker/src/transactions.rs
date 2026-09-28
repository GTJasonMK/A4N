use super::errors::WorkerFailure;
use sea_orm::DatabaseTransaction;

pub(super) async fn finish_transaction<T>(
    tx: DatabaseTransaction,
    result: Result<T, WorkerFailure>,
    operation: &str,
) -> Result<T, WorkerFailure> {
    match result {
        Ok(value) => {
            tx.commit()
                .await
                .map_err(|error| WorkerFailure::unavailable(error.to_string()))?;
            Ok(value)
        }
        Err(error) => {
            tx.rollback().await.map_err(|rollback_error| {
                WorkerFailure::unavailable(format!(
                    "{operation}失败：{}；事务回滚失败：{rollback_error}",
                    error.message
                ))
            })?;
            Err(error)
        }
    }
}
