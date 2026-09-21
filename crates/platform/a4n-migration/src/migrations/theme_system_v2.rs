//! Versioned theme documents, per-mode activation, preferences and resources.

use sea_orm::ConnectionTrait;
use sea_orm_migration::prelude::*;

const MIGRATION_NAME: &str = "theme_system_v2";

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
            "{MIGRATION_NAME} 不可逆：主题文档和资源合同不能安全回滚"
        )))
    }
}

const UP_SQL: &str = r#"
ALTER TABLE theme_config
    ADD COLUMN IF NOT EXISTS theme_document JSON NOT NULL DEFAULT '{"version":1,"tokens":{},"backgrounds":{}}';

UPDATE theme_config
SET theme_document = json_build_object(
    'version', 1,
    'tokens', css_variables::jsonb,
    'backgrounds', '{}'::jsonb
)
WHERE theme_document::jsonb = '{"version":1,"tokens":{},"backgrounds":{}}'::jsonb;

ALTER TABLE theme_config
    ADD CONSTRAINT ck_theme_config_document
        CHECK (
            json_typeof(theme_document) = 'object'
            AND (theme_document::jsonb ->> 'version') = '1'
            AND jsonb_typeof(theme_document::jsonb -> 'tokens') = 'object'
            AND jsonb_typeof(theme_document::jsonb -> 'backgrounds') = 'object'
        );

DROP INDEX IF EXISTS uidx_theme_config_active_user;
DROP INDEX IF EXISTS uidx_theme_config_active_global;
DROP INDEX IF EXISTS uidx_theme_config_system_default;

CREATE UNIQUE INDEX uidx_theme_config_active_user_mode
    ON theme_config (user_id, mode)
    WHERE user_id IS NOT NULL AND is_active;

CREATE UNIQUE INDEX uidx_theme_config_active_global_mode
    ON theme_config (mode)
    WHERE user_id IS NULL AND is_active;

CREATE UNIQUE INDEX uidx_theme_config_system_default_mode
    ON theme_config (mode)
    WHERE is_system_default;

CREATE TABLE IF NOT EXISTS theme_preference (
    user_id BIGINT PRIMARY KEY REFERENCES "user" (id) ON DELETE CASCADE,
    appearance TEXT NOT NULL DEFAULT 'system'
        CHECK (appearance IN ('system', 'light', 'dark')),
    created_at TIMESTAMP NOT NULL DEFAULT now(),
    updated_at TIMESTAMP NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS theme_resource (
    id BIGSERIAL PRIMARY KEY,
    theme_id BIGINT NOT NULL REFERENCES theme_config (id) ON DELETE CASCADE,
    resource_key TEXT NOT NULL,
    file_name TEXT NOT NULL,
    mime_type TEXT NOT NULL,
    file_path TEXT NOT NULL,
    sha256 TEXT NOT NULL,
    byte_size BIGINT NOT NULL CHECK (byte_size > 0 AND byte_size <= 8388608),
    created_at TIMESTAMP NOT NULL DEFAULT now(),
    updated_at TIMESTAMP NOT NULL DEFAULT now(),
    CONSTRAINT ck_theme_resource_key CHECK (
        resource_key <> ''
        AND resource_key = btrim(resource_key)
        AND resource_key !~ '[\\/\\.]'
    ),
    CONSTRAINT ck_theme_resource_file_name CHECK (
        file_name <> ''
        AND file_name = btrim(file_name)
        AND file_name !~ '[\\/\\\\]'
    ),
    CONSTRAINT ck_theme_resource_mime CHECK (mime_type IN ('image/png', 'image/jpeg', 'image/webp', 'image/gif'))
);

CREATE UNIQUE INDEX IF NOT EXISTS uidx_theme_resource_theme_key
    ON theme_resource (theme_id, resource_key);
CREATE INDEX IF NOT EXISTS idx_theme_resource_theme_id
    ON theme_resource (theme_id);
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use sea_orm::ConnectionTrait;

    #[tokio::test]
    async fn migration_allows_two_active_modes_for_one_user() {
        let db = crate::test_support::connect_test_database().await;
        db.execute_unprepared(
            "DROP TABLE IF EXISTS theme_resource, theme_preference, theme_config CASCADE;",
        )
        .await
        .unwrap();
        db.execute_unprepared(
            r#"
            CREATE TABLE theme_config (
                id BIGSERIAL PRIMARY KEY,
                user_id BIGINT,
                name TEXT NOT NULL,
                mode TEXT NOT NULL,
                css_variables JSON NOT NULL,
                is_system_default BOOLEAN NOT NULL DEFAULT FALSE,
                is_active BOOLEAN NOT NULL DEFAULT FALSE,
                created_at TIMESTAMP NOT NULL DEFAULT now(),
                updated_at TIMESTAMP NOT NULL DEFAULT now()
            );
            INSERT INTO theme_config (user_id, name, mode, css_variables, is_active)
            VALUES (7, 'Light', 'light', '{}', TRUE), (7, 'Dark', 'dark', '{}', FALSE);
            "#,
        )
        .await
        .unwrap();

        Migration.up(&SchemaManager::new(&db)).await.unwrap();
        db.execute_unprepared(
            "INSERT INTO theme_config (user_id, name, mode, css_variables, is_active) VALUES (7, 'Dark active', 'dark', '{}', TRUE)",
        ).await.unwrap();
        assert!(db.execute_unprepared(
            "INSERT INTO theme_config (user_id, name, mode, css_variables, is_active) VALUES (7, 'Second light', 'light', '{}', TRUE)",
        ).await.is_err());
    }
}
