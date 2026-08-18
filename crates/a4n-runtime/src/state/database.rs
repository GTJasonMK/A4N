use a4n_core::config::app_config::AppConfig;
use sea_orm::{
    ConnectOptions, ConnectionTrait, Database, DatabaseBackend, DatabaseConnection, Statement,
};
use std::time::Duration;

const GENERATION_OWNER_LOCK_NAMESPACE: i32 = 1_093_946_951;
const GENERATION_OWNER_LOCK_KEY: i32 = 1;

pub(super) async fn acquire_generation_owner_lock(
    database_url: &str,
) -> Result<DatabaseConnection, a4n_core::error::AppError> {
    let mut options = ConnectOptions::new(database_url.to_string());
    options
        .max_connections(1)
        .min_connections(1)
        .idle_timeout(Duration::from_secs(10 * 365 * 24 * 60 * 60))
        .max_lifetime(Duration::from_secs(10 * 365 * 24 * 60 * 60));
    let db = Database::connect(options)
        .await
        .map_err(a4n_core::error::AppError::Database)?;
    let row = db
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT pg_try_advisory_lock($1, $2) AS acquired",
            [
                GENERATION_OWNER_LOCK_NAMESPACE.into(),
                GENERATION_OWNER_LOCK_KEY.into(),
            ],
        ))
        .await
        .map_err(a4n_core::error::AppError::Database)?
        .ok_or_else(|| {
            a4n_core::error::AppError::ConfigError(
                "generation Host owner lock 查询未返回结果".to_string(),
            )
        })?;
    let acquired: bool = row
        .try_get("", "acquired")
        .map_err(a4n_core::error::AppError::Database)?;
    if !acquired {
        return Err(a4n_core::error::AppError::ConfigError(
            "同一 PostgreSQL 数据库已有 generation Host owner".to_string(),
        ));
    }
    Ok(db)
}

#[cfg(feature = "migration")]
pub(super) async fn run_startup_migrations(
    db: &DatabaseConnection,
) -> Result<(), a4n_core::error::AppError> {
    a4n_migration::migrator::run_migrations(db)
        .await
        .map_err(a4n_core::error::AppError::Database)
}

#[cfg(not(feature = "migration"))]
pub(super) async fn run_startup_migrations(
    db: &DatabaseConnection,
) -> Result<(), a4n_core::error::AppError> {
    let required = a4n_core::schema::REQUIRED_SCHEMA_MIGRATION;
    let applied = db
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT 1 AS applied FROM seaql_migrations WHERE version = $1 LIMIT 1",
            [required.into()],
        ))
        .await
        .map_err(|error| {
            a4n_core::error::AppError::ConfigError(format!(
                "无法验证数据库 schema 版本；当前 host 未编译 migration feature，请先运行 `a4n-migration`：{error}"
            ))
        })?;
    if applied.is_none() {
        return Err(a4n_core::error::AppError::ConfigError(format!(
            "数据库 schema 版本过旧：缺少必需迁移 `{required}`；当前 host 未编译 migration feature，请先运行 `a4n-migration`"
        )));
    }
    Ok(())
}

pub(super) fn validate_runtime_config(config: &AppConfig) -> Result<(), a4n_core::error::AppError> {
    a4n_core::config::env::validate_postgres_database_url(
        "A4N_DATABASE_URL",
        &config.database_url,
    )?;
    if config
        .security
        .jwt_secret
        .as_deref()
        .map(str::trim)
        .filter(|secret| !secret.is_empty())
        .is_none()
    {
        return Err(a4n_core::error::AppError::ConfigError(
            "A4N_JWT_SECRET 不能为空".to_string(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generation_owner_lock_key_is_fixed() {
        assert_eq!(GENERATION_OWNER_LOCK_NAMESPACE, 1_093_946_951);
        assert_eq!(GENERATION_OWNER_LOCK_KEY, 1);
    }
}
