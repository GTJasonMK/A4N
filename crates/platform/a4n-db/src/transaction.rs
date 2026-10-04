use futures::future::BoxFuture;

use crate::{DatabaseConnection, DbErr, DbTransaction, TransactionTrait};

pub type TransactionWork<'work, T, E = a4n_error::AppError> =
    BoxFuture<'work, (DbTransaction, Result<T, E>)>;

pub trait TransactionError: Sized {
    fn from_database(error: DbErr) -> Self;

    fn rollback_failure(operation: &str, original: Self, rollback_error: DbErr) -> Self;
}

impl TransactionError for a4n_error::AppError {
    fn from_database(error: DbErr) -> Self {
        Self::Database(error)
    }

    fn rollback_failure(operation: &str, original: Self, rollback_error: DbErr) -> Self {
        Self::Internal(anyhow::anyhow!(
            "{operation}失败：{original}；事务回滚失败：{rollback_error}"
        ))
    }
}

pub async fn run_in_transaction<'work, T, E>(
    db: &DatabaseConnection,
    operation: &str,
    work: impl FnOnce(DbTransaction) -> TransactionWork<'work, T, E>,
) -> Result<T, E>
where
    T: Send,
    E: TransactionError + Send,
{
    let tx = db.begin().await.map_err(E::from_database)?;
    let (tx, result) = work(tx).await;
    match result {
        Ok(value) => {
            tx.commit().await.map_err(E::from_database)?;
            Ok(value)
        }
        Err(error) => match tx.rollback().await {
            Ok(()) => Err(error),
            Err(rollback_error) => Err(E::rollback_failure(operation, error, rollback_error)),
        },
    }
}
