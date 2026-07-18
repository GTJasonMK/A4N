use a4n_core::config::app_config::AppConfig;
use sea_orm::DatabaseConnection;
#[cfg(not(feature = "migration"))]
use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};

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
    if !config.database_url.starts_with("postgres://")
        && !config.database_url.starts_with("postgresql://")
    {
        return Err(a4n_core::error::AppError::ConfigError(
            "A4N_DATABASE_URL 必须是 postgres:// 或 postgresql:// 连接串".to_string(),
        ));
    }
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
