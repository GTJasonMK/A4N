use futures::future::BoxFuture;

use crate::{DbExecutor, DbTransaction};

pub type TransactionWork<'work, T, E = a4n_error::AppError> =
    BoxFuture<'work, (DbTransaction, Result<T, E>)>;

pub trait TransactionError: Sized {
    fn from_database(error: a4n_error::AppError) -> Self;

    fn rollback_failure(
        operation: &str,
        original: Self,
        rollback_error: a4n_error::AppError,
    ) -> Self;
}

impl TransactionError for a4n_error::AppError {
    fn from_database(error: a4n_error::AppError) -> Self {
        error
    }

    fn rollback_failure(
        operation: &str,
        original: Self,
        rollback_error: a4n_error::AppError,
    ) -> Self {
        Self::Internal(anyhow::anyhow!(
            "{operation}失败：{original}；事务回滚失败：{rollback_error}"
        ))
    }
}

pub async fn run_in_transaction<'work, T, E>(
    db: &impl DbExecutor,
    operation: &str,
    work: impl FnOnce(DbTransaction) -> TransactionWork<'work, T, E>,
) -> Result<T, E>
where
    T: Send,
    E: TransactionError + Send,
{
    let tx = db
        .begin_db_transaction()
        .await
        .map_err(a4n_error::AppError::from)
        .map_err(E::from_database)?;
    let (tx, result) = work(tx).await;
    match result {
        Ok(value) => {
            tx.commit()
                .await
                .map_err(a4n_error::AppError::from)
                .map_err(E::from_database)?;
            Ok(value)
        }
        Err(error) => match tx.rollback().await {
            Ok(()) => Err(error),
            Err(rollback_error) => Err(E::rollback_failure(
                operation,
                error,
                a4n_error::AppError::from(rollback_error),
            )),
        },
    }
}
