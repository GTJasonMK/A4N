use std::{sync::Arc, time::Duration};

use sea_orm::{
    ConnectOptions, ConnectionTrait, Database, DatabaseBackend, Statement, TransactionTrait,
};

pub type DatabaseConnection = sea_orm::DatabaseConnection;
pub type DbTransaction = sea_orm::DatabaseTransaction;
pub type SharedDatabaseConnection = Arc<DatabaseConnection>;

#[derive(Clone)]
pub struct DbHandle {
    connection: SharedDatabaseConnection,
}

impl DbHandle {
    pub fn new(connection: SharedDatabaseConnection) -> Self {
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
            .map_err(a4n_error::AppError::Database)
    }

    pub fn connection(&self) -> &DatabaseConnection {
        self.connection.as_ref()
    }

    pub fn shared_connection(&self) -> SharedDatabaseConnection {
        Arc::clone(&self.connection)
    }
}

const GENERATION_OWNER_LOCK_NAMESPACE: i32 = 1_093_946_951;
const GENERATION_OWNER_LOCK_KEY: i32 = 1;

pub async fn acquire_generation_owner_lock(
    database_url: &str,
) -> Result<DatabaseConnection, a4n_error::AppError> {
    let mut options = ConnectOptions::new(database_url.to_string());
    options
        .max_connections(1)
        .min_connections(1)
        .idle_timeout(Duration::from_secs(10 * 365 * 24 * 60 * 60))
        .max_lifetime(Duration::from_secs(10 * 365 * 24 * 60 * 60));
    let database = Database::connect(options)
        .await
        .map_err(a4n_error::AppError::Database)?;
    let row = database
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT pg_try_advisory_lock($1, $2) AS acquired",
            [
                GENERATION_OWNER_LOCK_NAMESPACE.into(),
                GENERATION_OWNER_LOCK_KEY.into(),
            ],
        ))
        .await
        .map_err(a4n_error::AppError::Database)?
        .ok_or_else(|| {
            a4n_error::AppError::ConfigError(
                "generation Host owner lock 查询未返回结果".to_string(),
            )
        })?;
    let acquired: bool = row
        .try_get("", "acquired")
        .map_err(a4n_error::AppError::Database)?;
    if !acquired {
        return Err(a4n_error::AppError::ConfigError(
            "同一 PostgreSQL 数据库已有 generation Host owner".to_string(),
        ));
    }
    Ok(database)
}

pub async fn verify_required_schema(
    database: &DatabaseConnection,
) -> Result<(), a4n_error::AppError> {
    let required = a4n_db_models::REQUIRED_SCHEMA_MIGRATION;
    let applied = database
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

pub async fn verify_canvas_project_split(
    database: &DatabaseConnection,
) -> Result<(), a4n_error::AppError> {
    let row = database
        .query_one(Statement::from_string(
            DatabaseBackend::Postgres,
            r#"
SELECT
  (SELECT COUNT(*) FROM novel_project WHERE project_kind = 'video' AND root_project_id IS NULL AND source_snapshot_id IS NULL) AS legacy_freeform,
  (SELECT COUNT(*) FROM novel_project p WHERE p.project_kind = 'video' AND p.root_project_id IS NOT NULL AND p.source_snapshot_id IS NOT NULL AND (
      EXISTS (SELECT 1 FROM canvas_graph g JOIN canvas_node n ON n.graph_id = g.id WHERE g.project_id = p.id)
      OR EXISTS (SELECT 1 FROM canvas_text_asset t WHERE t.project_id = p.id)
      OR EXISTS (SELECT 1 FROM generation_spec s WHERE s.project_id = p.id AND s.source_context_kind = 'freeform')
      OR EXISTS (SELECT 1 FROM generated_image i WHERE i.project_id = p.id AND i.image_type IN ('canvas_image','canvas_frame'))
      OR EXISTS (SELECT 1 FROM generated_video v WHERE v.project_id = p.id AND (v.video_type IN ('canvas_video','canvas_assembly') OR v.workflow_mode IN ('canvas_video','canvas_trim','canvas_assembly')))
  )) AS derived_canvas
"#
            .to_string(),
        ))
        .await
        .map_err(a4n_error::AppError::Database)?
        .ok_or_else(|| a4n_error::AppError::ConfigError("Canvas 项目拆分预检无结果".into()))?;
    let legacy_freeform: i64 = row
        .try_get("", "legacy_freeform")
        .map_err(a4n_error::AppError::Database)?;
    let derived_canvas: i64 = row
        .try_get("", "derived_canvas")
        .map_err(a4n_error::AppError::Database)?;
    if legacy_freeform > 0 || derived_canvas > 0 {
        return Err(a4n_error::AppError::ConfigError(format!(
            "Canvas 项目拆分尚未完成：{} 个旧 freeform 项目、{} 个派生视频含画布内容；请运行维护迁移",
            legacy_freeform, derived_canvas
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::{DatabaseConnection, DbHandle};

    #[test]
    fn generation_owner_lock_key_is_fixed() {
        assert_eq!(super::GENERATION_OWNER_LOCK_NAMESPACE, 1_093_946_951);
        assert_eq!(super::GENERATION_OWNER_LOCK_KEY, 1);
    }

    #[tokio::test]
    async fn ping_reports_disconnected_database() {
        let database = DbHandle::new(Arc::new(DatabaseConnection::Disconnected));
        assert!(database.ping().await.is_err());
        assert!(database.begin().await.is_err());
    }
}
