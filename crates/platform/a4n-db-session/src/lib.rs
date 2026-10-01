use std::{ops::Deref, sync::Arc, time::Duration};

use sea_orm::{
    AccessMode, ConnectOptions, ConnectionTrait, Database, DatabaseBackend, DatabaseTransaction,
    DbErr, ExecResult, IsolationLevel, QueryResult, Statement, TransactionTrait,
};

pub type DatabaseConnection = sea_orm::DatabaseConnection;
pub type DbTransaction = DatabaseTransaction;
type SharedDatabaseConnection = Arc<DatabaseConnection>;

#[derive(Clone)]
pub struct DbHandle {
    connection: SharedDatabaseConnection,
}

impl DbHandle {
    pub fn new(connection: Arc<DatabaseConnection>) -> Self {
        Self { connection }
    }

    pub async fn connect(database_url: impl Into<String>) -> anyhow::Result<Self> {
        let mut options = ConnectOptions::new(database_url.into());
        options
            .max_connections(8)
            .min_connections(1)
            .connect_timeout(Duration::from_secs(5));
        let connection = Database::connect(options).await?;
        Ok(Self::new(Arc::new(connection)))
    }

    pub async fn connect_with_default_options(
        database_url: impl Into<String>,
    ) -> Result<Self, a4n_error::AppError> {
        let connection = Database::connect(ConnectOptions::new(database_url.into()))
            .await
            .map_err(a4n_error::AppError::database)?;
        Ok(Self::new(Arc::new(connection)))
    }

    pub async fn ping(&self) -> anyhow::Result<()> {
        self.connection
            .execute(Statement::from_string(
                DatabaseBackend::Postgres,
                "SELECT 1",
            ))
            .await?;
        Ok(())
    }

    pub async fn begin(&self) -> Result<DbTransaction, a4n_error::AppError> {
        self.connection
            .begin()
            .await
            .map_err(a4n_error::AppError::database)
    }

    pub async fn begin_read_only_repeatable_read(
        &self,
    ) -> Result<DbTransaction, a4n_error::AppError> {
        self.connection
            .begin_with_config(
                Some(IsolationLevel::RepeatableRead),
                Some(AccessMode::ReadOnly),
            )
            .await
            .map_err(a4n_error::AppError::database)
    }

    pub async fn verify_required_schema(&self) -> Result<(), a4n_error::AppError> {
        let required = a4n_db_models::REQUIRED_SCHEMA_MIGRATION;
        let applied = self
            .connection
            .query_one(Statement::from_sql_and_values(
                DatabaseBackend::Postgres,
                "SELECT 1 AS applied FROM seaql_migrations WHERE version = $1 LIMIT 1",
                [required.into()],
            ))
            .await
            .map_err(|error| {
                a4n_error::AppError::ConfigError(format!(
                    "无法验证数据库 schema 版本；当前 host 未编译 migration feature，请先运行 `a4n-migration`：{error}"
                ))
            })?;
        if applied.is_none() {
            return Err(a4n_error::AppError::ConfigError(format!(
                "数据库 schema 版本过旧：缺少必需迁移 `{required}`；当前 host 未编译 migration feature，请先运行 `a4n-migration`"
            )));
        }
        Ok(())
    }

    #[cfg(feature = "migration")]
    pub async fn run_migrations(&self) -> Result<(), a4n_error::AppError> {
        a4n_migration::migrator::run_migrations(&self.connection)
            .await
            .map_err(a4n_error::AppError::database)
    }
}

impl Deref for DbHandle {
    type Target = DatabaseConnection;

    fn deref(&self) -> &Self::Target {
        self.connection.as_ref()
    }
}

#[async_trait::async_trait]
impl ConnectionTrait for DbHandle {
    fn get_database_backend(&self) -> DatabaseBackend {
        self.connection.get_database_backend()
    }

    fn support_returning(&self) -> bool {
        self.connection.support_returning()
    }

    fn is_mock_connection(&self) -> bool {
        self.connection.is_mock_connection()
    }

    async fn execute(&self, statement: Statement) -> Result<ExecResult, DbErr> {
        self.connection.execute(statement).await
    }

    async fn execute_unprepared(&self, sql: &str) -> Result<ExecResult, DbErr> {
        self.connection.execute_unprepared(sql).await
    }

    async fn query_one(&self, statement: Statement) -> Result<Option<QueryResult>, DbErr> {
        self.connection.query_one(statement).await
    }

    async fn query_all(&self, statement: Statement) -> Result<Vec<QueryResult>, DbErr> {
        self.connection.query_all(statement).await
    }
}

#[async_trait::async_trait]
impl TransactionTrait for DbHandle {
    async fn begin(&self) -> Result<DatabaseTransaction, DbErr> {
        self.connection.begin().await
    }

    async fn begin_with_config(
        &self,
        isolation_level: Option<IsolationLevel>,
        access_mode: Option<AccessMode>,
    ) -> Result<DatabaseTransaction, DbErr> {
        self.connection
            .begin_with_config(isolation_level, access_mode)
            .await
    }

    async fn transaction<F, T, E>(&self, callback: F) -> Result<T, sea_orm::TransactionError<E>>
    where
        F: for<'c> FnOnce(
                &'c DatabaseTransaction,
            ) -> std::pin::Pin<
                Box<dyn std::future::Future<Output = Result<T, E>> + Send + 'c>,
            > + Send,
        T: Send,
        E: std::fmt::Display + std::fmt::Debug + Send,
    {
        self.connection.transaction(callback).await
    }

    async fn transaction_with_config<F, T, E>(
        &self,
        callback: F,
        isolation_level: Option<IsolationLevel>,
        access_mode: Option<AccessMode>,
    ) -> Result<T, sea_orm::TransactionError<E>>
    where
        F: for<'c> FnOnce(
                &'c DatabaseTransaction,
            ) -> std::pin::Pin<
                Box<dyn std::future::Future<Output = Result<T, E>> + Send + 'c>,
            > + Send,
        T: Send,
        E: std::fmt::Display + std::fmt::Debug + Send,
    {
        self.connection
            .transaction_with_config(callback, isolation_level, access_mode)
            .await
    }
}

pub async fn acquire_generation_owner_lock(
    database_url: &str,
) -> Result<DbHandle, a4n_error::AppError> {
    const LOCK_NAMESPACE: i32 = 1_093_946_951;
    const LOCK_KEY: i32 = 1;

    let mut options = ConnectOptions::new(database_url.to_string());
    options
        .max_connections(1)
        .min_connections(1)
        .idle_timeout(Duration::from_secs(10 * 365 * 24 * 60 * 60))
        .max_lifetime(Duration::from_secs(10 * 365 * 24 * 60 * 60));
    let connection = Database::connect(options)
        .await
        .map_err(a4n_error::AppError::database)?;
    let row = connection
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT pg_try_advisory_lock($1, $2) AS acquired",
            [LOCK_NAMESPACE.into(), LOCK_KEY.into()],
        ))
        .await
        .map_err(a4n_error::AppError::database)?
        .ok_or_else(|| {
            a4n_error::AppError::ConfigError(
                "generation Host owner lock 查询未返回结果".to_string(),
            )
        })?;
    let acquired: bool = row
        .try_get("", "acquired")
        .map_err(a4n_error::AppError::database)?;
    if !acquired {
        return Err(a4n_error::AppError::ConfigError(
            "同一 PostgreSQL 数据库已有 generation Host owner".to_string(),
        ));
    }
    Ok(DbHandle::new(Arc::new(connection)))
}
