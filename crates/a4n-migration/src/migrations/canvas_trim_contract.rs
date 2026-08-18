//! Canonicalize persisted canvas video trim configuration.

use sea_orm::{ConnectionTrait, DbBackend};
use sea_orm_migration::prelude::*;

const MIGRATION_NAME: &str = "canvas_trim_contract";

#[derive(DeriveMigrationName)]
pub struct Migration;

#[sea_orm_migration::async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() != DbBackend::Postgres {
            return Err(DbErr::Custom(format!(
                "A4N {MIGRATION_NAME} migration only supports PostgreSQL"
            )));
        }
        manager.get_connection().execute_unprepared(UP_SQL).await?;
        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Err(DbErr::Custom(
            "canvas_trim_contract 不可逆：显式 start_secs 不会恢复为运行时默认值".to_string(),
        ))
    }
}

const UP_SQL: &str = r#"
DO $$
BEGIN
    IF to_regclass(current_schema() || '.canvas_node') IS NULL THEN
        RAISE EXCEPTION 'canvas_trim_contract: canvas_node table is missing';
    END IF;

    IF EXISTS (
        SELECT 1
        FROM canvas_node
        WHERE node_kind = 'TrimVideoNode'
          AND jsonb_typeof(config_json::jsonb) IS DISTINCT FROM 'object'
    ) THEN
        RAISE EXCEPTION
            'canvas_trim_contract: TrimVideoNode config_json must be an object';
    END IF;

    IF EXISTS (
        SELECT 1
        FROM canvas_node
        WHERE node_kind = 'TrimVideoNode'
          AND config_json::jsonb ? 'start_secs'
          AND NOT CASE
              WHEN jsonb_typeof(config_json::jsonb -> 'start_secs') = 'number'
              THEN (config_json::jsonb ->> 'start_secs')::numeric >= 0
               AND (config_json::jsonb ->> 'start_secs')::numeric <= 3.4028235e38
              ELSE FALSE
          END
    ) THEN
        RAISE EXCEPTION
            'canvas_trim_contract: explicit start_secs must be a finite non-negative f32';
    END IF;

    UPDATE canvas_node
    SET config_json = jsonb_set(
        config_json::jsonb,
        '{start_secs}',
        '0'::jsonb,
        true
    )::json
    WHERE node_kind = 'TrimVideoNode'
      AND NOT config_json::jsonb ? 'start_secs';
END
$$;
"#;
