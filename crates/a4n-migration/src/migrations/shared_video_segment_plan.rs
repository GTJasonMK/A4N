//! Promote legacy mode-specific video segment plans to the shared segment plan identity.

use sea_orm::{ConnectionTrait, DbBackend, Statement};
use sea_orm_migration::prelude::*;

const MIGRATION_NAME: &str = "shared_video_segment_plan";

const PROMOTE_LEGACY_SEGMENT_PLANS_SQL: &str = r#"
WITH legacy AS (
    SELECT DISTINCT ON (project_id, chapter_id)
        *
    FROM chapter_video_prompt legacy_prompt
    WHERE legacy_prompt.workflow_mode IN ('first_frame_chain', 'chat_video')
      AND legacy_prompt.prompt_type IN ('first_frame_segment_plan', 'chat_video_segment_plan')
      AND legacy_prompt.shot_number = 0
      AND legacy_prompt.status = 'completed'
      AND NOT EXISTS (
          SELECT 1
          FROM chapter_video_prompt shared_prompt
          WHERE shared_prompt.project_id = legacy_prompt.project_id
            AND shared_prompt.chapter_id = legacy_prompt.chapter_id
            AND shared_prompt.workflow_mode = 'boundary_frames'
            AND shared_prompt.prompt_type = 'frame_segment_plan'
            AND shared_prompt.shot_number = 0
            AND shared_prompt.status = 'completed'
      )
    ORDER BY project_id, chapter_id, updated_at DESC, id DESC
)
INSERT INTO chapter_video_prompt (
    project_id,
    chapter_id,
    style,
    prompt_type,
    workflow_mode,
    shot_number,
    title,
    scene_summary,
    visual_prompt,
    motion_prompt,
    negative_prompt,
    duration_secs,
    aspect_ratio,
    reference_image_ids_json,
    status,
    generated_video_id,
    metadata_json,
    created_at,
    updated_at
)
SELECT
    project_id,
    chapter_id,
    style,
    'frame_segment_plan',
    'boundary_frames',
    0,
    '视频片段规划',
    scene_summary,
    visual_prompt,
    motion_prompt,
    negative_prompt,
    duration_secs,
    aspect_ratio,
    reference_image_ids_json,
    status,
    generated_video_id,
    (
        COALESCE(metadata_json::jsonb, '{}'::jsonb)
        || jsonb_build_object(
            'contract_version', 'director_continuity_v4',
            'a4n_migration', jsonb_build_object(
                'name', 'shared_video_segment_plan',
                'legacy_prompt_id', id,
                'legacy_prompt_type', prompt_type,
                'legacy_workflow_mode', workflow_mode
            )
        )
    )::json,
    created_at,
    NOW()
FROM legacy
"#;

const DELETE_PROMOTED_SEGMENT_PLANS_SQL: &str = r#"
DELETE FROM chapter_video_prompt
WHERE workflow_mode = 'boundary_frames'
  AND prompt_type = 'frame_segment_plan'
  AND shot_number = 0
  AND metadata_json->'a4n_migration'->>'name' = 'shared_video_segment_plan'
"#;

const UPDATE_DOWNSTREAM_SOURCE_PROMPT_TYPES_SQL: &str = r#"
WITH legacy_links AS (
    SELECT DISTINCT
        legacy_prompt.project_id,
        legacy_prompt.chapter_id,
        legacy_prompt.workflow_mode AS legacy_workflow_mode,
        legacy_prompt.prompt_type AS legacy_prompt_type
    FROM chapter_video_prompt legacy_prompt
    WHERE legacy_prompt.workflow_mode IN ('first_frame_chain', 'chat_video')
      AND legacy_prompt.prompt_type IN ('first_frame_segment_plan', 'chat_video_segment_plan')
      AND legacy_prompt.shot_number = 0
      AND legacy_prompt.status = 'completed'
      AND EXISTS (
          SELECT 1
          FROM chapter_video_prompt shared_prompt
          WHERE shared_prompt.project_id = legacy_prompt.project_id
            AND shared_prompt.chapter_id = legacy_prompt.chapter_id
            AND shared_prompt.workflow_mode = 'boundary_frames'
            AND shared_prompt.prompt_type = 'frame_segment_plan'
            AND shared_prompt.shot_number = 0
            AND shared_prompt.status = 'completed'
      )
)
UPDATE chapter_video_prompt child_prompt
SET
    metadata_json = (
        COALESCE(child_prompt.metadata_json::jsonb, '{}'::jsonb)
        || jsonb_build_object(
            'source_prompt_type',
            'frame_segment_plan',
            'a4n_migration_shared_segment_plan_source',
            jsonb_build_object(
                'name',
                'shared_video_segment_plan',
                'previous_source_prompt_type',
                child_prompt.metadata_json->>'source_prompt_type',
                'legacy_workflow_mode',
                legacy_links.legacy_workflow_mode
            )
        )
    )::json,
    updated_at = NOW()
FROM legacy_links
WHERE child_prompt.project_id = legacy_links.project_id
  AND child_prompt.chapter_id = legacy_links.chapter_id
  AND child_prompt.workflow_mode = legacy_links.legacy_workflow_mode
  AND child_prompt.metadata_json->>'source_prompt_type' = legacy_links.legacy_prompt_type
  AND NOT (
      COALESCE(child_prompt.metadata_json::jsonb, '{}'::jsonb)
      ? 'a4n_migration_shared_segment_plan_source'
  )
  AND (
      (
          legacy_links.legacy_prompt_type = 'first_frame_segment_plan'
          AND child_prompt.prompt_type IN ('first_frame_prompt', 'video_prompt')
      )
      OR (
          legacy_links.legacy_prompt_type = 'chat_video_segment_plan'
          AND child_prompt.prompt_type IN ('chat_video_frame_prompt', 'video_prompt')
      )
  )
"#;

const RESTORE_DOWNSTREAM_SOURCE_PROMPT_TYPES_SQL: &str = r#"
UPDATE chapter_video_prompt
SET
    metadata_json = (
        (
            COALESCE(metadata_json::jsonb, '{}'::jsonb)
            || jsonb_build_object(
                'source_prompt_type',
                metadata_json->'a4n_migration_shared_segment_plan_source'->>'previous_source_prompt_type'
            )
        )
        - 'a4n_migration_shared_segment_plan_source'
    )::json,
    updated_at = NOW()
WHERE metadata_json->'a4n_migration_shared_segment_plan_source'->>'name' = 'shared_video_segment_plan'
  AND metadata_json->'a4n_migration_shared_segment_plan_source'->>'previous_source_prompt_type' IS NOT NULL
"#;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        ensure_postgres(manager)?;
        execute_pg_sql(manager, PROMOTE_LEGACY_SEGMENT_PLANS_SQL).await?;
        execute_pg_sql(manager, UPDATE_DOWNSTREAM_SOURCE_PROMPT_TYPES_SQL).await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        ensure_postgres(manager)?;
        execute_pg_sql(manager, RESTORE_DOWNSTREAM_SOURCE_PROMPT_TYPES_SQL).await?;
        execute_pg_sql(manager, DELETE_PROMOTED_SEGMENT_PLANS_SQL).await
    }
}

fn ensure_postgres(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    if manager.get_database_backend() != DbBackend::Postgres {
        return Err(DbErr::Custom(format!(
            "A4N {MIGRATION_NAME} migration only supports PostgreSQL"
        )));
    }
    Ok(())
}

async fn execute_pg_sql(manager: &SchemaManager<'_>, sql: &str) -> Result<(), DbErr> {
    manager
        .get_connection()
        .execute(Statement::from_string(DbBackend::Postgres, sql.to_string()))
        .await?;
    Ok(())
}
