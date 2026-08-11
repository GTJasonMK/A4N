//! Enforce the single persisted contract for theme records.

use sea_orm::ConnectionTrait;
use sea_orm_migration::prelude::*;

const MIGRATION_NAME: &str = "theme_config_integrity_contract";

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
            "{MIGRATION_NAME} 不可逆：主题读写只支持收敛后的唯一合同"
        )))
    }
}

const UP_SQL: &str = r#"
DO $$
BEGIN
    IF to_regclass(current_schema() || '.theme_config') IS NULL THEN
        RAISE EXCEPTION 'theme_config_integrity_contract: theme_config table is missing';
    END IF;

    IF EXISTS (
        SELECT 1
        FROM theme_config
        WHERE name = ''
           OR name <> btrim(name)
           OR mode NOT IN ('light', 'dark')
           OR json_typeof(css_variables) IS DISTINCT FROM 'object'
           OR jsonb_path_exists(
               css_variables::jsonb,
               '$.* ? (@.type() != "string")'
           )
           OR (is_system_default AND user_id IS NOT NULL)
    ) THEN
        RAISE EXCEPTION 'theme_config_integrity_contract: invalid theme row';
    END IF;

    IF EXISTS (
        SELECT 1
        FROM theme_config
        GROUP BY user_id, name
        HAVING count(*) > 1
    ) THEN
        RAISE EXCEPTION 'theme_config_integrity_contract: duplicate theme scope/name';
    END IF;

    IF EXISTS (
        SELECT 1
        FROM theme_config
        WHERE is_active
        GROUP BY user_id
        HAVING count(*) > 1
    ) THEN
        RAISE EXCEPTION 'theme_config_integrity_contract: duplicate active theme scope';
    END IF;

    IF (SELECT count(*) FROM theme_config WHERE is_system_default) > 1 THEN
        RAISE EXCEPTION 'theme_config_integrity_contract: duplicate system default theme';
    END IF;
END $$;

ALTER TABLE theme_config
    ADD CONSTRAINT ck_theme_config_name
        CHECK (name <> '' AND name = btrim(name)),
    ADD CONSTRAINT ck_theme_config_mode
        CHECK (mode IN ('light', 'dark')),
    ADD CONSTRAINT ck_theme_config_css_variables
        CHECK (
            json_typeof(css_variables) = 'object'
            AND NOT jsonb_path_exists(
                css_variables::jsonb,
                '$.* ? (@.type() != "string")'
            )
        ),
    ADD CONSTRAINT ck_theme_config_system_default_scope
        CHECK (NOT is_system_default OR user_id IS NULL);

CREATE UNIQUE INDEX uidx_theme_config_scope_name
    ON theme_config (user_id, name) NULLS NOT DISTINCT;

CREATE UNIQUE INDEX uidx_theme_config_active_user
    ON theme_config (user_id)
    WHERE user_id IS NOT NULL AND is_active;

CREATE UNIQUE INDEX uidx_theme_config_active_global
    ON theme_config ((is_active))
    WHERE user_id IS NULL AND is_active;

CREATE UNIQUE INDEX uidx_theme_config_system_default
    ON theme_config ((is_system_default))
    WHERE is_system_default;
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn migration_enforces_the_typed_theme_contract() {
        let db = crate::test_support::connect_test_database().await;
        db.execute_unprepared(
            r#"
            DROP TABLE IF EXISTS theme_config CASCADE;
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
            INSERT INTO theme_config (
                user_id,
                name,
                mode,
                css_variables,
                is_system_default,
                is_active
            ) VALUES
                (NULL, 'System Light', 'light', '{"--bg":"255 255 255"}', TRUE, TRUE),
                (7, 'User Dark', 'dark', '{"--bg":"0 0 0"}', FALSE, TRUE);
            "#,
        )
        .await
        .unwrap();

        Migration.up(&SchemaManager::new(&db)).await.unwrap();

        for invalid_insert in [
            "INSERT INTO theme_config (user_id, name, mode, css_variables) VALUES (7, 'User Dark', 'dark', '{}')",
            "INSERT INTO theme_config (user_id, name, mode, css_variables, is_active) VALUES (7, 'Second Active', 'light', '{}', TRUE)",
            "INSERT INTO theme_config (user_id, name, mode, css_variables, is_system_default) VALUES (NULL, 'Second Default', 'light', '{}', TRUE)",
            "INSERT INTO theme_config (user_id, name, mode, css_variables) VALUES (8, 'Bad Mode', 'system', '{}')",
            r#"INSERT INTO theme_config (user_id, name, mode, css_variables) VALUES (8, 'Bad CSS', 'light', '{"--radius":8}')"#,
            "INSERT INTO theme_config (user_id, name, mode, css_variables) VALUES (8, ' Bad Name ', 'light', '{}')",
            "INSERT INTO theme_config (user_id, name, mode, css_variables, is_system_default) VALUES (8, 'User Default', 'light', '{}', TRUE)",
        ] {
            assert!(db.execute_unprepared(invalid_insert).await.is_err());
        }
    }
}
