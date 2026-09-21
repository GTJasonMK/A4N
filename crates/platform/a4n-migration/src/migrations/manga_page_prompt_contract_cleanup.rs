//! 收敛漫画账本为每页唯一的 page_generation 记录。

use sea_orm::ConnectionTrait;
use sea_orm_migration::prelude::*;

const MIGRATION_NAME: &str = "manga_page_prompt_contract_cleanup";

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
            "{MIGRATION_NAME} 不可逆：回滚会恢复已删除的非页面与重复账本"
        )))
    }
}

const UP_SQL: &str = r#"
DO $$
DECLARE
    redundant_column_count INTEGER;
BEGIN
    IF to_regclass(current_schema() || '.chapter_manga_prompt') IS NULL THEN
        RAISE EXCEPTION
            'manga_page_prompt_contract_cleanup: chapter_manga_prompt table is missing';
    END IF;

    SELECT COUNT(*)
    INTO redundant_column_count
    FROM information_schema.columns
    WHERE table_schema = current_schema()
      AND table_name = 'chapter_manga_prompt'
      AND column_name IN ('panel_number', 'camera_angle', 'dialogue_position');

    IF redundant_column_count NOT IN (0, 3) THEN
        RAISE EXCEPTION
            'manga_page_prompt_contract_cleanup: partial redundant column schema';
    END IF;

    IF EXISTS (
        SELECT 1
        FROM chapter_manga_prompt
        WHERE prompt_type <> 'page_generation'
          AND generated_image_id IS NOT NULL
    ) THEN
        RAISE EXCEPTION
            'manga_page_prompt_contract_cleanup: redundant prompt rows still own generated images';
    END IF;

    IF EXISTS (
        SELECT 1
        FROM generation_spec AS spec
        JOIN chapter_manga_prompt AS prompt
          ON prompt.id = spec.target_entity_id
        WHERE spec.target_entity_kind = 'chapter_manga_prompt'
          AND prompt.prompt_type <> 'page_generation'
    ) THEN
        RAISE EXCEPTION
            'manga_page_prompt_contract_cleanup: redundant prompt rows still own generation specs';
    END IF;

    IF EXISTS (
        SELECT 1
        FROM chapter_manga_prompt
        WHERE prompt_type = 'page_generation'
        GROUP BY project_id, chapter_id, page_number
        HAVING COUNT(DISTINCT generated_image_id)
            FILTER (WHERE generated_image_id IS NOT NULL) > 1
    ) THEN
        RAISE EXCEPTION
            'manga_page_prompt_contract_cleanup: duplicate page rows own different generated images';
    END IF;

    IF EXISTS (
        SELECT 1
        FROM generation_spec AS spec
        JOIN (
            SELECT id
            FROM (
                SELECT
                    id,
                    ROW_NUMBER() OVER (
                        PARTITION BY project_id, chapter_id, page_number
                        ORDER BY
                            (generated_image_id IS NOT NULL) DESC,
                            updated_at DESC,
                            id DESC
                    ) AS duplicate_rank
                FROM chapter_manga_prompt
                WHERE prompt_type = 'page_generation'
            ) AS ranked
            WHERE duplicate_rank > 1
        ) AS duplicate_prompt
          ON duplicate_prompt.id = spec.target_entity_id
        WHERE spec.target_entity_kind = 'chapter_manga_prompt'
    ) THEN
        RAISE EXCEPTION
            'manga_page_prompt_contract_cleanup: duplicate page rows still own generation specs';
    END IF;

    IF EXISTS (
        SELECT 1
        FROM chapter_manga_prompt
        WHERE prompt_type = 'page_generation'
          AND page_number <= 0
    ) THEN
        RAISE EXCEPTION
            'manga_page_prompt_contract_cleanup: canonical prompt identity is invalid';
    END IF;

    IF redundant_column_count = 3 THEN
        IF EXISTS (
            SELECT 1
            FROM chapter_manga_prompt
            WHERE prompt_type = 'page_generation'
              AND panel_number <> 0
        ) THEN
            RAISE EXCEPTION
                'manga_page_prompt_contract_cleanup: canonical rows still use panel_number';
        END IF;
    END IF;
END $$;

WITH ranked AS (
    SELECT
        id,
        ROW_NUMBER() OVER (
            PARTITION BY project_id, chapter_id, page_number
            ORDER BY
                (generated_image_id IS NOT NULL) DESC,
                updated_at DESC,
                id DESC
        ) AS duplicate_rank
    FROM chapter_manga_prompt
    WHERE prompt_type = 'page_generation'
), obsolete AS (
    SELECT id
    FROM chapter_manga_prompt
    WHERE prompt_type <> 'page_generation'
    UNION ALL
    SELECT id
    FROM ranked
    WHERE duplicate_rank > 1
)
DELETE FROM generation_run_target AS target
USING obsolete
WHERE target.target_table = 'chapter_manga_prompt'
  AND target.target_id = obsolete.id;

WITH ranked AS (
    SELECT
        id,
        ROW_NUMBER() OVER (
            PARTITION BY project_id, chapter_id, page_number
            ORDER BY
                (generated_image_id IS NOT NULL) DESC,
                updated_at DESC,
                id DESC
        ) AS duplicate_rank
    FROM chapter_manga_prompt
    WHERE prompt_type = 'page_generation'
), obsolete AS (
    SELECT id
    FROM chapter_manga_prompt
    WHERE prompt_type <> 'page_generation'
    UNION ALL
    SELECT id
    FROM ranked
    WHERE duplicate_rank > 1
)
DELETE FROM chapter_manga_prompt AS prompt
USING obsolete
WHERE prompt.id = obsolete.id;

ALTER TABLE chapter_manga_prompt
    DROP COLUMN IF EXISTS panel_number,
    DROP COLUMN IF EXISTS camera_angle,
    DROP COLUMN IF EXISTS dialogue_position,
    DROP CONSTRAINT IF EXISTS chapter_manga_prompt_canonical_identity,
    ADD CONSTRAINT chapter_manga_prompt_canonical_identity CHECK (
        prompt_type = 'page_generation' AND page_number > 0
    );

CREATE UNIQUE INDEX IF NOT EXISTS idx_chapter_manga_prompt_identity_unique
ON chapter_manga_prompt (project_id, chapter_id, page_number);
"#;
