//! 收敛章节视频时间线早期实现遗留的重复列和 metadata 绑定。

use sea_orm::{ConnectionTrait, DbBackend};
use sea_orm_migration::prelude::*;

const MIGRATION_NAME: &str = "chapter_video_timeline_cleanup";

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
        manager
            .get_connection()
            .execute_unprepared(
                r#"
UPDATE chapter_video_prompt AS prompt
SET timeline_item_id = item.id
FROM chapter_video_timeline_item AS item
JOIN chapter_video_timeline AS timeline ON timeline.id = item.timeline_id
WHERE item.item_kind = 'inserted'
  AND item.insert_plan_prompt_id = prompt.id
  AND timeline.project_id = prompt.project_id
  AND timeline.chapter_id = prompt.chapter_id
  AND prompt.timeline_item_id IS NULL;

WITH candidate AS (
    SELECT
        video.id,
        video.project_id,
        video.chapter_id,
        COALESCE(
            video.metadata_json ->> 'timeline_item_id',
            video.metadata_json -> 'video_execution_identity' ->> 'timeline_item_id'
        ) AS timeline_item_id
    FROM generated_video AS video
    WHERE video.timeline_item_id IS NULL
)
UPDATE generated_video AS video
SET timeline_item_id = item.id
FROM candidate
JOIN chapter_video_timeline_item AS item
  ON item.id = CASE
      WHEN candidate.timeline_item_id ~ '^[1-9][0-9]*$'
      THEN candidate.timeline_item_id::BIGINT
      ELSE NULL
  END
JOIN chapter_video_timeline AS timeline
  ON timeline.id = item.timeline_id
 AND timeline.project_id = candidate.project_id
 AND timeline.chapter_id = candidate.chapter_id
WHERE video.id = candidate.id;

DO $$
BEGIN
    IF EXISTS (
        WITH identity AS (
            SELECT
                prompt.id,
                prompt.timeline_item_id,
                prompt.metadata_json::jsonb ->> 'timeline_item_id'
                    AS legacy_timeline_item_id,
                prompt.metadata_json::jsonb
                    -> 'video_execution_identity'
                    ->> 'timeline_item_id' AS execution_timeline_item_id,
                COALESCE(
                    prompt.metadata_json::jsonb ->> 'timeline_item_id',
                    prompt.metadata_json::jsonb
                        -> 'video_execution_identity'
                        ->> 'timeline_item_id'
                ) AS metadata_timeline_item_id,
                prompt.metadata_json::jsonb
                    -> 'video_execution_identity'
                    ->> 'segment_kind' AS execution_segment_kind
            FROM chapter_video_prompt AS prompt
        )
        SELECT 1
        FROM identity
        WHERE
            (
                legacy_timeline_item_id IS NOT NULL
                AND execution_timeline_item_id IS NOT NULL
                AND legacy_timeline_item_id IS DISTINCT FROM execution_timeline_item_id
            )
            OR (timeline_item_id IS NULL) IS DISTINCT FROM
                (metadata_timeline_item_id IS NULL)
            OR (
                metadata_timeline_item_id IS NOT NULL
                AND (
                    NOT (metadata_timeline_item_id ~ '^[1-9][0-9]*$')
                    OR timeline_item_id IS DISTINCT FROM CASE
                        WHEN metadata_timeline_item_id ~ '^[1-9][0-9]*$'
                        THEN metadata_timeline_item_id::BIGINT
                        ELSE NULL
                    END
                    OR execution_segment_kind IS DISTINCT FROM 'inserted'
                )
            )
    ) THEN
        RAISE EXCEPTION
            'chapter_video_timeline_cleanup: prompt metadata identity is missing or inconsistent';
    END IF;
    IF EXISTS (
        WITH identity AS (
            SELECT
                video.id,
                video.timeline_item_id,
                video.metadata_json::jsonb ->> 'timeline_item_id'
                    AS legacy_timeline_item_id,
                video.metadata_json::jsonb
                    -> 'video_execution_identity'
                    ->> 'timeline_item_id' AS execution_timeline_item_id,
                COALESCE(
                    video.metadata_json::jsonb ->> 'timeline_item_id',
                    video.metadata_json::jsonb
                        -> 'video_execution_identity'
                        ->> 'timeline_item_id'
                ) AS metadata_timeline_item_id,
                video.metadata_json::jsonb
                    -> 'video_execution_identity'
                    ->> 'segment_kind' AS execution_segment_kind
            FROM generated_video AS video
        )
        SELECT 1
        FROM identity
        WHERE
            (
                legacy_timeline_item_id IS NOT NULL
                AND execution_timeline_item_id IS NOT NULL
                AND legacy_timeline_item_id IS DISTINCT FROM execution_timeline_item_id
            )
            OR (timeline_item_id IS NULL) IS DISTINCT FROM
                (metadata_timeline_item_id IS NULL)
            OR (
                metadata_timeline_item_id IS NOT NULL
                AND (
                    NOT (metadata_timeline_item_id ~ '^[1-9][0-9]*$')
                    OR timeline_item_id IS DISTINCT FROM CASE
                        WHEN metadata_timeline_item_id ~ '^[1-9][0-9]*$'
                        THEN metadata_timeline_item_id::BIGINT
                        ELSE NULL
                    END
                    OR execution_segment_kind IS DISTINCT FROM 'inserted'
                )
            )
    ) THEN
        RAISE EXCEPTION
            'chapter_video_timeline_cleanup: generated video metadata identity is missing or inconsistent';
    END IF;
END $$;

UPDATE chapter_video_prompt
SET metadata_json = (
    metadata_json::jsonb - 'timeline_item_id' - 'segment_kind' - 'timeline_display_index'
)::json
WHERE metadata_json IS NOT NULL
  AND metadata_json::jsonb ?| ARRAY[
      'timeline_item_id',
      'segment_kind',
      'timeline_display_index'
  ];
UPDATE generated_video
SET metadata_json = (
    metadata_json::jsonb - 'timeline_item_id' - 'segment_kind' - 'timeline_display_index'
)::json
WHERE metadata_json IS NOT NULL
  AND metadata_json::jsonb ?| ARRAY[
      'timeline_item_id',
      'segment_kind',
      'timeline_display_index'
  ];

UPDATE chapter_video_prompt
SET metadata_json = jsonb_set(
    metadata_json::jsonb,
    '{video_execution_identity}',
    (metadata_json::jsonb -> 'video_execution_identity') - 'segment_number'
)::json
WHERE metadata_json IS NOT NULL
  AND metadata_json::jsonb -> 'video_execution_identity' ->> 'segment_kind' = 'inserted'
  AND (metadata_json::jsonb -> 'video_execution_identity') ? 'segment_number';
UPDATE generated_video
SET metadata_json = jsonb_set(
    metadata_json::jsonb,
    '{video_execution_identity}',
    (metadata_json::jsonb -> 'video_execution_identity') - 'segment_number'
)::json
WHERE metadata_json IS NOT NULL
  AND metadata_json::jsonb -> 'video_execution_identity' ->> 'segment_kind' = 'inserted'
  AND (metadata_json::jsonb -> 'video_execution_identity') ? 'segment_number';

ALTER TABLE chapter_video_prompt
    DROP CONSTRAINT IF EXISTS chapter_video_prompt_metadata_identity_removed,
    DROP CONSTRAINT IF EXISTS chapter_video_prompt_timeline_identity_consistent,
    ADD CONSTRAINT chapter_video_prompt_metadata_identity_removed CHECK (
        metadata_json IS NULL
        OR NOT (metadata_json::jsonb ?| ARRAY[
            'timeline_item_id',
            'segment_kind',
            'timeline_display_index'
        ])
        AND NOT (
            metadata_json::jsonb -> 'video_execution_identity' ->> 'segment_kind' = 'inserted'
            AND (metadata_json::jsonb -> 'video_execution_identity') ? 'segment_number'
        )
    ),
    ADD CONSTRAINT chapter_video_prompt_timeline_identity_consistent CHECK ((
        timeline_item_id IS NULL
        OR (
            metadata_json IS NOT NULL
            AND metadata_json::jsonb
                -> 'video_execution_identity'
                ->> 'segment_kind' = 'inserted'
            AND timeline_item_id = CASE
                WHEN metadata_json::jsonb
                    -> 'video_execution_identity'
                    ->> 'timeline_item_id' ~ '^[1-9][0-9]*$'
                THEN (
                    metadata_json::jsonb
                        -> 'video_execution_identity'
                        ->> 'timeline_item_id'
                )::BIGINT
                ELSE NULL
            END
        )
    ) IS TRUE);
ALTER TABLE generated_video
    DROP CONSTRAINT IF EXISTS generated_video_metadata_identity_removed,
    DROP CONSTRAINT IF EXISTS generated_video_timeline_identity_consistent,
    ADD CONSTRAINT generated_video_metadata_identity_removed CHECK (
        metadata_json IS NULL
        OR NOT (metadata_json::jsonb ?| ARRAY[
            'timeline_item_id',
            'segment_kind',
            'timeline_display_index'
        ])
        AND NOT (
            metadata_json::jsonb -> 'video_execution_identity' ->> 'segment_kind' = 'inserted'
            AND (metadata_json::jsonb -> 'video_execution_identity') ? 'segment_number'
        )
    ),
    ADD CONSTRAINT generated_video_timeline_identity_consistent CHECK ((
        timeline_item_id IS NULL
        OR (
            metadata_json IS NOT NULL
            AND metadata_json::jsonb
                -> 'video_execution_identity'
                ->> 'segment_kind' = 'inserted'
            AND timeline_item_id = CASE
                WHEN metadata_json::jsonb
                    -> 'video_execution_identity'
                    ->> 'timeline_item_id' ~ '^[1-9][0-9]*$'
                THEN (
                    metadata_json::jsonb
                        -> 'video_execution_identity'
                        ->> 'timeline_item_id'
                )::BIGINT
                ELSE NULL
            END
        )
    ) IS TRUE);

DO $$
BEGIN
    IF EXISTS (
        SELECT 1
        FROM information_schema.columns
        WHERE table_schema = current_schema()
          AND table_name = 'chapter_video_timeline_item'
          AND column_name = 'stale'
    ) THEN
        EXECUTE $sql$
            UPDATE chapter_video_timeline_item
            SET status = 'needs_reanchor', updated_at = NOW()
            WHERE item_kind = 'inserted'
              AND status = 'active'
              AND stale IS TRUE
        $sql$;
    END IF;
END $$;

ALTER TABLE chapter_video_timeline_item
    DROP COLUMN IF EXISTS stale,
    DROP COLUMN IF EXISTS created_by_user_id;

DO $$
BEGIN
    IF EXISTS (
        SELECT 1
        FROM chapter_video_timeline_item
        WHERE item_kind = 'inserted'
          AND insert_plan_prompt_id IS NULL
    ) THEN
        RAISE EXCEPTION
            'chapter_video_timeline_cleanup: inserted timeline item missing prompt binding';
    END IF;
    IF EXISTS (
        SELECT 1
        FROM chapter_video_prompt AS prompt
        JOIN chapter_video_timeline_item AS item ON item.id = prompt.timeline_item_id
        JOIN chapter_video_timeline AS timeline ON timeline.id = item.timeline_id
        WHERE prompt.timeline_item_id IS NOT NULL
          AND (
              item.item_kind <> 'inserted'
              OR item.insert_plan_prompt_id <> prompt.id
              OR timeline.project_id <> prompt.project_id
              OR timeline.chapter_id <> prompt.chapter_id
          )
    ) THEN
        RAISE EXCEPTION
            'chapter_video_timeline_cleanup: prompt timeline binding is inconsistent';
    END IF;
    IF EXISTS (
        SELECT 1
        FROM generated_video AS video
        JOIN chapter_video_timeline_item AS item ON item.id = video.timeline_item_id
        JOIN chapter_video_timeline AS timeline ON timeline.id = item.timeline_id
        WHERE video.timeline_item_id IS NOT NULL
          AND (
              item.item_kind <> 'inserted'
              OR timeline.project_id <> video.project_id
              OR timeline.chapter_id IS DISTINCT FROM video.chapter_id
          )
    ) THEN
        RAISE EXCEPTION
            'chapter_video_timeline_cleanup: generated video timeline binding is inconsistent';
    END IF;
END $$;

ALTER TABLE chapter_video_timeline_item
    DROP CONSTRAINT IF EXISTS chapter_video_timeline_item_identity_valid,
    DROP CONSTRAINT IF EXISTS chapter_video_timeline_item_insert_plan_prompt_id_fkey;
ALTER TABLE chapter_video_timeline_item
    ADD CONSTRAINT chapter_video_timeline_item_identity_valid CHECK (
        (item_kind = 'native'
            AND native_source_shot_id IS NOT NULL
            AND native_segment_index_in_shot IS NOT NULL
            AND native_segment_number IS NOT NULL
            AND insert_plan_prompt_id IS NULL)
        OR
        (item_kind = 'inserted'
            AND native_source_shot_id IS NULL
            AND native_segment_index_in_shot IS NULL
            AND native_segment_number IS NULL
            AND insert_plan_prompt_id IS NOT NULL)
    ),
    ADD CONSTRAINT chapter_video_timeline_item_insert_plan_prompt_id_fkey
        FOREIGN KEY (insert_plan_prompt_id)
        REFERENCES chapter_video_prompt(id)
        ON DELETE RESTRICT;

DROP INDEX IF EXISTS uidx_chapter_video_timeline_insert_prompt;
CREATE UNIQUE INDEX uidx_chapter_video_timeline_insert_prompt
    ON chapter_video_timeline_item (insert_plan_prompt_id)
    WHERE item_kind = 'inserted';
"#,
            )
            .await?;
        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Err(DbErr::Custom(
            "chapter_video_timeline_cleanup 不可逆：回滚会恢复重复身份来源".to_string(),
        ))
    }
}
