//! Backfill the card-based segment workflow without rewriting existing media.

use sea_orm::DbBackend;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[sea_orm_migration::async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() != DbBackend::Postgres {
            return Err(DbErr::Custom(
                "A4N segment_card_video_workflow migration only supports PostgreSQL".to_string(),
            ));
        }
        manager
            .get_connection()
            .execute_unprepared(BACKFILL_SQL)
            .await?;
        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Err(DbErr::Custom(
            "segment_card_video_workflow 不可逆：采用状态和片段媒体映射不能安全退回章节级状态"
                .to_string(),
        ))
    }
}

const BACKFILL_SQL: &str = r#"
UPDATE chapter_video_prompt
SET metadata_json = (
        COALESCE(metadata_json::jsonb, '{}'::jsonb)
        || jsonb_build_object(
            'guidance_mode', COALESCE(metadata_json->>'guidance_mode', 'system_frames')
        )
    )::json,
    updated_at = NOW()
WHERE prompt_type = 'video_prompt';

WITH ranked AS (
    SELECT
        prompt.generated_video_id AS video_id,
        prompt.shot_number,
        ROW_NUMBER() OVER (
            PARTITION BY prompt.project_id, prompt.chapter_id, prompt.shot_number
            ORDER BY prompt.updated_at DESC NULLS LAST, prompt.id DESC
        ) AS candidate_rank
    FROM chapter_video_prompt prompt
    JOIN generated_video video ON video.id = prompt.generated_video_id
    WHERE prompt.prompt_type = 'video_prompt'
      AND prompt.generated_video_id IS NOT NULL
      AND video.status = 'completed'
      AND NULLIF(BTRIM(video.file_path), '') IS NOT NULL
)
UPDATE generated_video video
SET metadata_json = (
        COALESCE(video.metadata_json::jsonb, '{}'::jsonb)
        || jsonb_build_object(
            'shot_number', ranked.shot_number,
            'selected_for_segment', true,
            'selected_for_segment_at', COALESCE(video.updated_at, video.created_at, NOW())
        )
    )::json
FROM ranked
WHERE ranked.video_id = video.id
  AND ranked.candidate_rank = 1;

UPDATE generated_video video
SET metadata_json = (
        COALESCE(video.metadata_json::jsonb, '{}'::jsonb)
        || jsonb_build_object(
            'shot_number', prompt.shot_number,
            'segment_mapping_status', 'mapped'
        )
    )::json
FROM chapter_video_prompt prompt
WHERE video.video_type = 'reference_media_video'
  AND video.project_id = prompt.project_id
  AND video.chapter_id = prompt.chapter_id
  AND video.metadata_json->>'prompt_id' = prompt.id::text
  AND prompt.prompt_type = 'video_prompt';

UPDATE generated_video
SET metadata_json = (
        COALESCE(metadata_json::jsonb, '{}'::jsonb)
        || jsonb_build_object(
            'segment_mapping_status', 'pending_manual_assignment',
            'segment_mapping_reason', 'legacy_reference_media_without_segment_identity'
        )
    )::json
WHERE video_type = 'reference_media_video'
  AND COALESCE(metadata_json->>'segment_mapping_status', '') = '';
"#;

#[cfg(test)]
mod tests {
    use super::BACKFILL_SQL;

    #[test]
    fn migration_keeps_unmapped_reference_media_explicit() {
        assert!(BACKFILL_SQL.contains("pending_manual_assignment"));
        assert!(BACKFILL_SQL.contains("selected_for_segment"));
        assert!(BACKFILL_SQL.contains("guidance_mode"));
    }
}
