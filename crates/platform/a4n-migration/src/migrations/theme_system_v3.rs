//! Remove the superseded flat theme token column after document migration.

use sea_orm::ConnectionTrait;
use sea_orm_migration::prelude::*;

const MIGRATION_NAME: &str = "theme_system_v3";

#[derive(DeriveMigrationName)]
pub struct Migration;

#[sea_orm_migration::async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(
                "ALTER TABLE theme_config DROP CONSTRAINT IF EXISTS ck_theme_config_css_variables; \
                 ALTER TABLE theme_config DROP COLUMN IF EXISTS css_variables",
            )
            .await?;
        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Err(DbErr::Custom(format!(
            "{MIGRATION_NAME} 不可逆：旧主题 Token 列已由 versioned document 取代"
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sea_orm::{ConnectionTrait, Statement};

    #[tokio::test]
    async fn migration_removes_flat_theme_tokens_without_touching_document() {
        let db = crate::test_support::connect_test_database().await;
        db.execute_unprepared(
            r#"
            DROP TABLE IF EXISTS theme_resource, theme_preference, theme_config CASCADE;
            CREATE TABLE theme_config (
                id BIGSERIAL PRIMARY KEY,
                user_id BIGINT,
                name TEXT NOT NULL,
                mode TEXT NOT NULL,
                css_variables JSON NOT NULL,
                theme_document JSON NOT NULL,
                CONSTRAINT ck_theme_config_css_variables
                    CHECK (json_typeof(css_variables) = 'object'),
                is_system_default BOOLEAN NOT NULL DEFAULT FALSE,
                is_active BOOLEAN NOT NULL DEFAULT FALSE,
                created_at TIMESTAMP NOT NULL DEFAULT now(),
                updated_at TIMESTAMP NOT NULL DEFAULT now()
            );
            INSERT INTO theme_config (name, mode, css_variables, theme_document)
            VALUES ('Dark', 'dark', '{"--old":"0 0 0"}', '{"version":1,"tokens":{},"backgrounds":{}}');
            "#,
        )
        .await
        .unwrap();

        Migration.up(&SchemaManager::new(&db)).await.unwrap();

        assert!(
            db.query_one(Statement::from_string(
                db.get_database_backend(),
                "SELECT theme_document FROM theme_config LIMIT 1".to_string(),
            ))
            .await
            .is_ok()
        );
        assert!(
            db.query_one(Statement::from_string(
                db.get_database_backend(),
                "SELECT css_variables FROM theme_config LIMIT 1".to_string(),
            ))
            .await
            .is_err()
        );
    }
}
