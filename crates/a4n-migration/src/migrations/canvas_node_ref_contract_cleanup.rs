//! Canonicalize canvas node references and remove legacy reference formats.

use sea_orm::{ConnectionTrait, DbBackend};
use sea_orm_migration::prelude::*;

const MIGRATION_NAME: &str = "canvas_node_ref_contract_cleanup";

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
            "canvas_node_ref_contract_cleanup 不可逆：回滚会恢复已删除的旧画布引用格式".to_string(),
        ))
    }
}

const UP_SQL: &str = r#"
DO $$
BEGIN
    IF to_regclass(current_schema() || '.canvas_node') IS NULL THEN
        RAISE EXCEPTION 'canvas_node_ref_contract_cleanup: canvas_node table is missing';
    END IF;

    IF EXISTS (
        SELECT 1
        FROM canvas_node
        WHERE ref_type = 'legacy_chapter_video_prompt'
    ) THEN
        RAISE EXCEPTION
            'canvas_node_ref_contract_cleanup: legacy chapter-video-prompt refs require explicit resolution';
    END IF;

    IF EXISTS (
        SELECT 1
        FROM canvas_node
        WHERE (
            config_json::jsonb ? 'legacy_generated_image_id'
            AND NOT CASE
                WHEN jsonb_typeof(config_json::jsonb -> 'legacy_generated_image_id') = 'number'
                 AND (config_json::jsonb ->> 'legacy_generated_image_id') ~ '^[1-9][0-9]*$'
                THEN (config_json::jsonb ->> 'legacy_generated_image_id')::numeric
                    <= 9223372036854775807
                ELSE FALSE
            END
        ) OR (
            config_json::jsonb ? 'legacy_generated_video_id'
            AND NOT CASE
                WHEN jsonb_typeof(config_json::jsonb -> 'legacy_generated_video_id') = 'number'
                 AND (config_json::jsonb ->> 'legacy_generated_video_id') ~ '^[1-9][0-9]*$'
                THEN (config_json::jsonb ->> 'legacy_generated_video_id')::numeric
                    <= 9223372036854775807
                ELSE FALSE
            END
        )
    ) THEN
        RAISE EXCEPTION
            'canvas_node_ref_contract_cleanup: legacy config refs must be positive int64 values';
    END IF;

    IF EXISTS (
        SELECT 1
        FROM canvas_node
        WHERE (
            config_json::jsonb ? 'legacy_generated_image_id'
            AND config_json::jsonb ? 'generated_image_id'
            AND config_json::jsonb -> 'legacy_generated_image_id'
                IS DISTINCT FROM config_json::jsonb -> 'generated_image_id'
        ) OR (
            config_json::jsonb ? 'legacy_generated_video_id'
            AND config_json::jsonb ? 'generated_video_id'
            AND config_json::jsonb -> 'legacy_generated_video_id'
                IS DISTINCT FROM config_json::jsonb -> 'generated_video_id'
        )
    ) THEN
        RAISE EXCEPTION
            'canvas_node_ref_contract_cleanup: legacy and canonical config refs diverge';
    END IF;

    UPDATE canvas_node
    SET ref_type = CASE ref_type
        WHEN 'legacy_generated_image' THEN 'generated_image'
        WHEN 'legacy_generated_video' THEN 'generated_video'
        ELSE ref_type
    END
    WHERE ref_type IN ('legacy_generated_image', 'legacy_generated_video');

    UPDATE canvas_node
    SET config_json = jsonb_set(
        config_json::jsonb,
        '{generated_image_id}',
        config_json::jsonb -> 'legacy_generated_image_id',
        true
    )::json
    WHERE config_json::jsonb ? 'legacy_generated_image_id'
      AND NOT config_json::jsonb ? 'generated_image_id';

    UPDATE canvas_node
    SET config_json = jsonb_set(
        config_json::jsonb,
        '{generated_video_id}',
        config_json::jsonb -> 'legacy_generated_video_id',
        true
    )::json
    WHERE config_json::jsonb ? 'legacy_generated_video_id'
      AND NOT config_json::jsonb ? 'generated_video_id';

    UPDATE canvas_node
    SET config_json = (
        config_json::jsonb
            - 'legacy_generated_image_id'
            - 'legacy_generated_video_id'
    )::json
    WHERE config_json::jsonb ? 'legacy_generated_image_id'
       OR config_json::jsonb ? 'legacy_generated_video_id';

    IF EXISTS (
        SELECT 1
        FROM canvas_node
        WHERE ref_type IS NOT NULL
          AND ref_type NOT IN (
              'inline',
              'config',
              'canvas_text_asset',
              'prompt_artifact',
              'media_asset',
              'video_segment',
              'assembly_timeline',
              'generation_spec',
              'generated_image',
              'generated_video'
          )
    ) THEN
        RAISE EXCEPTION
            'canvas_node_ref_contract_cleanup: unknown canvas node ref_type remains';
    END IF;

    IF EXISTS (
        SELECT 1
        FROM canvas_node
        WHERE NOT CASE
            WHEN ref_type IS NULL THEN ref_id IS NULL
            WHEN ref_type = 'inline' THEN ref_id IS NULL
            WHEN ref_type = 'config' THEN ref_id IS NULL OR ref_id > 0
            WHEN ref_type IN (
                    'canvas_text_asset',
                    'prompt_artifact',
                    'media_asset',
                    'video_segment',
                    'assembly_timeline',
                    'generation_spec',
                    'generated_image',
                    'generated_video'
                ) THEN ref_id IS NOT NULL AND ref_id > 0
            ELSE FALSE
        END
    ) THEN
        RAISE EXCEPTION
            'canvas_node_ref_contract_cleanup: invalid canonical ref_type/ref_id shape remains';
    END IF;

    ALTER TABLE canvas_node
        DROP CONSTRAINT IF EXISTS canvas_node_ref_contract_check;
    ALTER TABLE canvas_node
        ADD CONSTRAINT canvas_node_ref_contract_check CHECK (
            CASE
                WHEN ref_type IS NULL THEN ref_id IS NULL
                WHEN ref_type = 'inline' THEN ref_id IS NULL
                WHEN ref_type = 'config' THEN ref_id IS NULL OR ref_id > 0
                WHEN ref_type IN (
                    'canvas_text_asset',
                    'prompt_artifact',
                    'media_asset',
                    'video_segment',
                    'assembly_timeline',
                    'generation_spec',
                    'generated_image',
                    'generated_video'
                ) THEN ref_id IS NOT NULL AND ref_id > 0
                ELSE FALSE
            END
        );
END
$$;
"#;
