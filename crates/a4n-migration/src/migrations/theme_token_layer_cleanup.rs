//! Normalize the retired aggregate surface opacity token into layer tokens.

use sea_orm::ConnectionTrait;
use sea_orm_migration::prelude::*;

const MIGRATION_NAME: &str = "theme_token_layer_cleanup";

#[derive(DeriveMigrationName)]
pub struct Migration;

#[sea_orm_migration::async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.get_connection().execute_unprepared(UP_SQL).await?;
        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Err(DbErr::Custom(format!(
            "{MIGRATION_NAME} 不可逆：主题层级透明度已取代旧总透明度 Token"
        )))
    }
}

const UP_SQL: &str = r#"
WITH legacy AS (
    SELECT
        id,
        theme_document::jsonb AS document,
        theme_document::jsonb -> 'tokens' AS tokens,
        CASE
            WHEN (
                theme_document::jsonb -> 'tokens' ->> '--a4n-surface-opacity'
            ) ~ '^(0|1|0[.][0-9]+|1[.]0+)$'
            THEN to_jsonb(
                theme_document::jsonb -> 'tokens' ->> '--a4n-surface-opacity'
            )
        END AS opacity
    FROM theme_config
    WHERE theme_document::jsonb -> 'tokens' ? '--a4n-surface-opacity'
)
UPDATE theme_config AS theme
SET theme_document = jsonb_set(
    legacy.document,
    '{tokens}',
    (legacy.tokens - '--a4n-surface-opacity')
        || jsonb_strip_nulls(jsonb_build_object(
            '--a4n-workspace-opacity',
            COALESCE(
                legacy.tokens -> '--a4n-workspace-opacity',
                legacy.opacity
            ),
            '--a4n-panel-opacity',
            COALESCE(legacy.tokens -> '--a4n-panel-opacity', legacy.opacity),
            '--a4n-card-opacity',
            COALESCE(legacy.tokens -> '--a4n-card-opacity', legacy.opacity),
            '--a4n-control-opacity',
            COALESCE(legacy.tokens -> '--a4n-control-opacity', legacy.opacity),
            '--a4n-dialog-opacity',
            COALESCE(legacy.tokens -> '--a4n-dialog-opacity', legacy.opacity)
        )),
    true
)::json
FROM legacy
WHERE theme.id = legacy.id;
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use sea_orm::{ConnectionTrait, Statement};

    #[tokio::test]
    async fn migration_expands_legacy_surface_opacity_without_overwriting_layers() {
        let db = crate::test_support::connect_test_database().await;
        db.execute_unprepared(
            r#"
            DROP TABLE IF EXISTS theme_config CASCADE;
            CREATE TABLE theme_config (
                id BIGSERIAL PRIMARY KEY,
                theme_document JSON NOT NULL
            );
            INSERT INTO theme_config (theme_document)
            VALUES
                ('{"version":1,"tokens":{"--a4n-surface-opacity":"0.2","--a4n-workspace-opacity":"0.7"},"backgrounds":{}}'),
                ('{"version":1,"tokens":{"--a4n-surface-opacity":"0.3"},"backgrounds":{}}');
            "#,
        )
        .await
        .unwrap();

        Migration.up(&SchemaManager::new(&db)).await.unwrap();

        let rows = db
            .query_all(Statement::from_string(
                db.get_database_backend(),
                "SELECT theme_document FROM theme_config ORDER BY id".to_string(),
            ))
            .await
            .unwrap();
        let first: serde_json::Value = rows[0].try_get("", "theme_document").unwrap();
        let second: serde_json::Value = rows[1].try_get("", "theme_document").unwrap();

        assert_eq!(first["tokens"]["--a4n-workspace-opacity"], "0.7");
        assert_eq!(first["tokens"]["--a4n-dialog-opacity"], "0.2");
        assert!(first["tokens"].get("--a4n-surface-opacity").is_none());
        assert_eq!(second["tokens"]["--a4n-card-opacity"], "0.3");
        assert!(second["tokens"].get("--a4n-surface-opacity").is_none());
    }
}
