//! Complete the segment recipe model and remove the superseded video-selection flag.

use sea_orm::DbBackend;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[sea_orm_migration::async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() != DbBackend::Postgres {
            return Err(DbErr::Custom(
                "A4N segment_execution_recipe_identity migration only supports PostgreSQL"
                    .to_string(),
            ));
        }
        manager.get_connection().execute_unprepared(UP_SQL).await?;
        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Err(DbErr::Custom(
            "segment_execution_recipe_identity 不可逆：旧的双重采用状态不会恢复".to_string(),
        ))
    }
}

const UP_SQL: &str = r#"
UPDATE chapter_video_prompt AS plan
SET metadata_json = jsonb_set(
        COALESCE(plan.metadata_json::jsonb, '{}'::jsonb),
        '{segment_recipes}',
        (
            SELECT jsonb_agg(
                (
                    jsonb_build_object(
                        'schema_version', 'video_segment_execution_recipe_v1',
                        'segment_number', (segment.value->>'segment_number')::integer,
                        'workflow_mode', 'chat_video',
                        'style', BTRIM(plan.style),
                        'prompt_output_language', 'zh_cn',
                        'duration_secs', (segment.value->>'duration_secs')::integer,
                        'duration_secs_options', jsonb_build_array((segment.value->>'duration_secs')::integer),
                        'aspect_ratio', segment.value->>'aspect_ratio',
                        'reference_image_ids', '[]'::jsonb,
                        'video_library_image_ids', '[]'::jsonb,
                        'guidance_mode', 'none',
                        'guidance_reference_image_ids', '[]'::jsonb,
                        'continuity_source_video_id', NULL,
                        'reference_media', NULL,
                        'adopted_video_id', NULL
                    )
                    || COALESCE(existing.value, '{}'::jsonb)
                    || jsonb_build_object(
                        'schema_version', 'video_segment_execution_recipe_v1',
                        'segment_number', (segment.value->>'segment_number')::integer,
                        'style', BTRIM(plan.style),
                        'duration_secs', (segment.value->>'duration_secs')::integer,
                        'duration_secs_options', jsonb_build_array((segment.value->>'duration_secs')::integer),
                        'aspect_ratio', segment.value->>'aspect_ratio',
                        'adopted_video_id', COALESCE(
                            existing.value->'adopted_video_id',
                            to_jsonb(legacy_adopted.video_id)
                        )
                    )
                )
                ORDER BY (segment.value->>'segment_number')::integer
            )
            FROM jsonb_array_elements(plan.metadata_json::jsonb->'segments') AS segment(value)
            LEFT JOIN LATERAL (
                SELECT recipe.value
                FROM jsonb_array_elements(
                    CASE
                        WHEN jsonb_typeof(plan.metadata_json::jsonb->'segment_recipes') = 'array'
                            THEN plan.metadata_json::jsonb->'segment_recipes'
                        ELSE '[]'::jsonb
                    END
                ) AS recipe(value)
                WHERE recipe.value->>'segment_number' = segment.value->>'segment_number'
                LIMIT 1
            ) AS existing ON TRUE
            LEFT JOIN LATERAL (
                SELECT video.id AS video_id
                FROM generated_video video
                WHERE video.project_id = plan.project_id
                  AND video.chapter_id = plan.chapter_id
                  AND video.status = 'completed'
                  AND NULLIF(BTRIM(video.file_path), '') IS NOT NULL
                  AND COALESCE((video.metadata_json::jsonb->>'selected_for_segment')::boolean, false)
                  AND (video.metadata_json::jsonb->>'shot_number')::integer
                      = (segment.value->>'segment_number')::integer
                  AND (
                      (video.metadata_json::jsonb->'video_execution_identity'->>'segment_plan_prompt_id')::bigint = plan.id
                      OR (
                          video.metadata_json::jsonb->'video_execution_identity'->>'segment_plan_prompt_id' IS NULL
                          AND plan.id = (
                              SELECT MAX(current_plan.id)
                              FROM chapter_video_prompt current_plan
                              WHERE current_plan.project_id = plan.project_id
                                AND current_plan.chapter_id = plan.chapter_id
                                AND current_plan.prompt_type = 'frame_segment_plan'
                                AND current_plan.workflow_mode = 'shared_segment_plan'
                                AND current_plan.status = 'completed'
                          )
                      )
                  )
                ORDER BY
                    NULLIF(video.metadata_json::jsonb->>'selected_for_segment_at', '') DESC NULLS LAST,
                    video.updated_at DESC NULLS LAST,
                    video.id DESC
                LIMIT 1
            ) AS legacy_adopted ON TRUE
        ),
        true
    )::json,
    updated_at = NOW()
WHERE plan.prompt_type = 'frame_segment_plan'
  AND plan.workflow_mode = 'shared_segment_plan'
  AND plan.status = 'completed'
  AND NULLIF(BTRIM(plan.style), '') IS NOT NULL
  AND jsonb_typeof(plan.metadata_json::jsonb->'segments') = 'array'
  AND jsonb_array_length(plan.metadata_json::jsonb->'segments') > 0;

UPDATE generated_video
SET metadata_json = (
        COALESCE(metadata_json::jsonb, '{}'::jsonb)
        - 'selected_for_segment'
        - 'selected_for_segment_at'
    )::json,
    updated_at = NOW()
WHERE metadata_json::jsonb ? 'selected_for_segment'
   OR metadata_json::jsonb ? 'selected_for_segment_at';
"#;

#[cfg(test)]
mod tests {
    use super::UP_SQL;

    #[test]
    fn migration_backfills_complete_recipes_and_removes_legacy_selection() {
        assert!(UP_SQL.contains("segment_recipes"));
        assert!(UP_SQL.contains("video_segment_execution_recipe_v1"));
        assert!(UP_SQL.contains("duration_secs_options"));
        assert!(UP_SQL.contains("legacy_adopted.video_id"));
        assert!(UP_SQL.contains("segment_plan_prompt_id"));
        assert!(UP_SQL.contains("- 'selected_for_segment'"));
    }
}
