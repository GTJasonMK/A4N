//! Canonicalize persisted canvas frame extraction configuration.

use sea_orm::{ConnectionTrait, DbBackend};
use sea_orm_migration::prelude::*;

const MIGRATION_NAME: &str = "canvas_extract_frame_contract";

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
            "canvas_extract_frame_contract 不可逆：旧抽帧别名与隐式默认值不会恢复".to_string(),
        ))
    }
}

const UP_SQL: &str = r#"
DO $$
BEGIN
    IF to_regclass(current_schema() || '.canvas_node') IS NULL THEN
        RAISE EXCEPTION 'canvas_extract_frame_contract: canvas_node table is missing';
    END IF;

    IF EXISTS (
        SELECT 1
        FROM canvas_node
        WHERE node_kind = 'ExtractFrameNode'
          AND jsonb_typeof(config_json::jsonb) <> 'object'
    ) THEN
        RAISE EXCEPTION
            'canvas_extract_frame_contract: ExtractFrameNode config_json must be an object';
    END IF;

    IF EXISTS (
        SELECT 1
        FROM canvas_node
        WHERE node_kind = 'ExtractFrameNode'
          AND COALESCE(
                NULLIF(btrim(config_json::jsonb ->> 'frame_kind'), ''),
                NULLIF(btrim(config_json::jsonb ->> 'kind'), ''),
                'first'
              ) NOT IN (
                'first', 'first_frame',
                'tail', 'last', 'last_frame', 'end',
                'time', 'key', 'key_frame', 'at_secs'
              )
    ) THEN
        RAISE EXCEPTION
            'canvas_extract_frame_contract: unknown ExtractFrameNode frame kind remains';
    END IF;

    WITH canonical AS (
        SELECT id,
               CASE COALESCE(
                   NULLIF(btrim(config_json::jsonb ->> 'frame_kind'), ''),
                   NULLIF(btrim(config_json::jsonb ->> 'kind'), ''),
                   'first'
               )
                   WHEN 'first' THEN 'first'
                   WHEN 'first_frame' THEN 'first'
                   WHEN 'tail' THEN 'tail'
                   WHEN 'last' THEN 'tail'
                   WHEN 'last_frame' THEN 'tail'
                   WHEN 'end' THEN 'tail'
                   WHEN 'time' THEN 'time'
                   WHEN 'key' THEN 'time'
                   WHEN 'key_frame' THEN 'time'
                   WHEN 'at_secs' THEN 'time'
               END AS frame_kind
        FROM canvas_node
        WHERE node_kind = 'ExtractFrameNode'
    )
    UPDATE canvas_node node
    SET config_json = jsonb_set(
        node.config_json::jsonb - 'kind' - 'frame_kind',
        '{frame_kind}',
        to_jsonb(canonical.frame_kind),
        true
    )::json
    FROM canonical
    WHERE node.id = canonical.id;

    UPDATE canvas_node
    SET config_json = (config_json::jsonb - 'at_secs')::json
    WHERE node_kind = 'ExtractFrameNode'
      AND config_json::jsonb ->> 'frame_kind' IN ('first', 'tail')
      AND config_json::jsonb ? 'at_secs';

    IF EXISTS (
        SELECT 1
        FROM canvas_node
        WHERE node_kind = 'ExtractFrameNode'
          AND config_json::jsonb ->> 'frame_kind' = 'time'
          AND NOT CASE
              WHEN jsonb_typeof(config_json::jsonb -> 'at_secs') = 'number'
              THEN (config_json::jsonb ->> 'at_secs')::numeric >= 0
               AND (config_json::jsonb ->> 'at_secs')::numeric <= 3.4028235e38
              ELSE FALSE
          END
    ) THEN
        RAISE EXCEPTION
            'canvas_extract_frame_contract: time extraction requires a finite non-negative at_secs';
    END IF;
END
$$;
"#;
