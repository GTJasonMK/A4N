//! 删除章节视频时间线未消费的状态与 JSON 身份镜像。

use sea_orm::{ConnectionTrait, DbBackend};
use sea_orm_migration::prelude::*;

const MIGRATION_NAME: &str = "chapter_video_timeline_state_cleanup";

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
        Err(DbErr::Custom(format!(
            "{MIGRATION_NAME} 不可逆：已删除未消费的重复状态"
        )))
    }
}

const UP_SQL: &str = r#"
UPDATE chapter_video_prompt
SET metadata_json = jsonb_set(
    metadata_json::jsonb,
    '{video_execution_identity}',
    jsonb_build_object(
        'contract_epoch', metadata_json::jsonb -> 'video_execution_identity' -> 'contract_epoch',
        'source_content_sha256', metadata_json::jsonb -> 'video_execution_identity' -> 'source_content_sha256',
        'segment_sha256', metadata_json::jsonb -> 'video_execution_identity' -> 'segment_sha256'
    )
)::json
WHERE timeline_item_id IS NOT NULL
  AND metadata_json::jsonb -> 'video_execution_identity' ?& ARRAY[
      'contract_epoch', 'source_content_sha256', 'segment_sha256'
  ];
UPDATE generated_video
SET metadata_json = jsonb_set(
    metadata_json::jsonb,
    '{video_execution_identity}',
    jsonb_build_object(
        'contract_epoch', metadata_json::jsonb -> 'video_execution_identity' -> 'contract_epoch',
        'source_content_sha256', metadata_json::jsonb -> 'video_execution_identity' -> 'source_content_sha256',
        'segment_sha256', metadata_json::jsonb -> 'video_execution_identity' -> 'segment_sha256'
    )
)::json
WHERE timeline_item_id IS NOT NULL
  AND metadata_json::jsonb -> 'video_execution_identity' ?& ARRAY[
      'contract_epoch', 'source_content_sha256', 'segment_sha256'
  ];

UPDATE generated_video
SET metadata_json = jsonb_set(
    metadata_json::jsonb,
    '{provider_request_snapshot}',
    (metadata_json::jsonb -> 'provider_request_snapshot')
        - 'video_execution_identity'
        - 'source_prompt_type'
        - 'workflow_mode'
)::json
WHERE metadata_json::jsonb -> 'provider_request_snapshot' IS NOT NULL;
UPDATE generated_video
SET metadata_json = (metadata_json::jsonb - 'generation_asset_fingerprint')::json
WHERE timeline_item_id IS NOT NULL
  AND metadata_json::jsonb ? 'generation_asset_fingerprint';

UPDATE chapter_video_prompt
SET metadata_json = (
    metadata_json::jsonb
        - 'segment_plan_prompt_id'
        - 'segment_number'
        - 'guidance_mode'
        - 'keyframe_image_ids'
        - 'continuity_source'
        - 'segment_recipe_fingerprint'
)::json
WHERE metadata_json IS NOT NULL;
UPDATE chapter_video_prompt
SET metadata_json = (metadata_json::jsonb - 'generation_asset_fingerprint')::json
WHERE timeline_item_id IS NOT NULL
  AND metadata_json::jsonb ? 'generation_asset_fingerprint';

ALTER TABLE chapter_video_prompt
    DROP CONSTRAINT IF EXISTS chapter_video_prompt_metadata_identity_removed,
    DROP CONSTRAINT IF EXISTS chapter_video_prompt_timeline_identity_consistent,
    ADD CONSTRAINT chapter_video_prompt_metadata_identity_removed CHECK ((
        metadata_json IS NULL
        OR (
            NOT (metadata_json::jsonb ?| ARRAY[
                'timeline_item_id',
                'segment_kind',
                'timeline_display_index',
                'segment_plan_prompt_id',
                'segment_number',
                'segment_recipe_fingerprint',
                'guidance_mode',
                'keyframe_image_ids',
                'continuity_source'
            ])
            AND (
                timeline_item_id IS NULL
                OR (
                    jsonb_typeof(metadata_json::jsonb -> 'video_execution_identity') = 'object'
                    AND metadata_json::jsonb -> 'video_execution_identity' ?& ARRAY[
                        'contract_epoch',
                        'source_content_sha256',
                        'segment_sha256'
                    ]
                    AND (metadata_json::jsonb -> 'video_execution_identity') - ARRAY[
                        'contract_epoch',
                        'source_content_sha256',
                        'segment_sha256'
                    ] = '{}'::jsonb
                    AND metadata_json::jsonb -> 'video_execution_identity' -> 'contract_epoch' = '5'::jsonb
                    AND metadata_json::jsonb -> 'video_execution_identity' ->> 'source_content_sha256' ~ '^[0-9a-f]{64}$'
                    AND metadata_json::jsonb -> 'video_execution_identity' ->> 'segment_sha256' ~ '^[0-9a-f]{64}$'
                    AND NOT (metadata_json::jsonb ? 'generation_asset_fingerprint')
                )
            )
        )
    ) IS TRUE),
    ADD CONSTRAINT chapter_video_prompt_timeline_identity_consistent CHECK ((
        timeline_item_id IS NULL
        OR (
            metadata_json IS NOT NULL
            AND metadata_json::jsonb ->> 'source_prompt_type' = 'inserted_segment'
            AND jsonb_typeof(metadata_json::jsonb -> 'segment_recipe') = 'object'
        )
    ) IS TRUE);

ALTER TABLE generated_video
    DROP CONSTRAINT IF EXISTS generated_video_metadata_identity_removed,
    DROP CONSTRAINT IF EXISTS generated_video_timeline_identity_consistent,
    ADD CONSTRAINT generated_video_metadata_identity_removed CHECK ((
        metadata_json IS NULL
        OR (
            NOT (metadata_json::jsonb ?| ARRAY[
                'timeline_item_id',
                'segment_kind',
                'timeline_display_index'
            ])
            AND (
                timeline_item_id IS NULL
                OR (
                    jsonb_typeof(metadata_json::jsonb -> 'video_execution_identity') = 'object'
                    AND metadata_json::jsonb -> 'video_execution_identity' ?& ARRAY[
                        'contract_epoch',
                        'source_content_sha256',
                        'segment_sha256'
                    ]
                    AND (metadata_json::jsonb -> 'video_execution_identity') - ARRAY[
                        'contract_epoch',
                        'source_content_sha256',
                        'segment_sha256'
                    ] = '{}'::jsonb
                    AND metadata_json::jsonb -> 'video_execution_identity' -> 'contract_epoch' = '5'::jsonb
                    AND metadata_json::jsonb -> 'video_execution_identity' ->> 'source_content_sha256' ~ '^[0-9a-f]{64}$'
                    AND metadata_json::jsonb -> 'video_execution_identity' ->> 'segment_sha256' ~ '^[0-9a-f]{64}$'
                    AND NOT (metadata_json::jsonb ? 'generation_asset_fingerprint')
                )
            )
            AND NOT (
                COALESCE(
                    metadata_json::jsonb -> 'provider_request_snapshot',
                    '{}'::jsonb
                ) ?| ARRAY[
                    'video_execution_identity',
                    'source_prompt_type',
                    'workflow_mode'
                ]
            )
        )
    ) IS TRUE),
    ADD CONSTRAINT generated_video_timeline_identity_consistent CHECK ((
        timeline_item_id IS NULL
        OR (
            metadata_json IS NOT NULL
            AND metadata_json::jsonb ->> 'source_prompt_type' = 'inserted_segment'
        )
    ) IS TRUE);

"#;

#[cfg(test)]
mod tests {
    use super::{Migration, UP_SQL};
    use sea_orm::{ConnectionTrait, Statement};
    use sea_orm_migration::prelude::{MigrationName, MigrationTrait, SchemaManager};

    #[test]
    fn cleanup_is_a_single_forward_metadata_migration() {
        assert_eq!(Migration.name(), "chapter_video_timeline_state_cleanup");
        assert!(UP_SQL.contains("metadata_json::jsonb ? 'generation_asset_fingerprint'"));
        assert!(UP_SQL.contains("'contract_epoch' = '5'::jsonb"));
        assert!(UP_SQL.contains("'^[0-9a-f]{64}$'"));
        assert!(!UP_SQL.contains("jsonb_object_length"));
        assert!(!UP_SQL.contains("CREATE TABLE"));
        assert!(!UP_SQL.contains("chapter_video_timeline_item"));
    }

    #[tokio::test]
    async fn final_schema_has_one_timeline_state_source() {
        let db = crate::test_support::connect_test_database().await;
        let row = db
            .query_one(Statement::from_string(
                db.get_database_backend(),
                "SELECT NOT EXISTS (SELECT 1 FROM chapter_video_prompt \
                            WHERE metadata_json::jsonb ?| ARRAY[ \
                                'segment_plan_prompt_id', 'segment_number', 'guidance_mode', \
                                'keyframe_image_ids', 'continuity_source', \
                                'segment_recipe_fingerprint' \
                            ] \
                            OR (timeline_item_id IS NOT NULL \
                                AND metadata_json::jsonb ? 'generation_asset_fingerprint')) \
                            AS prompt_recipe_mirror_removed, \
                        NOT EXISTS (SELECT 1 FROM generated_video \
                            WHERE COALESCE( \
                                metadata_json::jsonb -> 'provider_request_snapshot', '{}'::jsonb \
                            ) ?| ARRAY[ \
                                'video_execution_identity', 'source_prompt_type', 'workflow_mode' \
                            ] \
                            OR (timeline_item_id IS NOT NULL \
                                AND metadata_json::jsonb ? 'generation_asset_fingerprint')) \
                            AS video_mirrors_removed"
                    .to_string(),
            ))
            .await
            .expect("查询时间线收敛 schema")
            .expect("时间线收敛 schema 查询应返回一行");

        assert!(
            row.try_get::<bool>("", "prompt_recipe_mirror_removed")
                .unwrap()
        );
        assert!(row.try_get::<bool>("", "video_mirrors_removed").unwrap());

        let error = Migration
            .down(&SchemaManager::new(&db))
            .await
            .expect_err("migration is irreversible");
        assert!(error.to_string().contains("不可逆"));
    }
}
