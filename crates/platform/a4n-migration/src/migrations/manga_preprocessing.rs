//! Adds the chapter-level preprocessing artifact used before media adaptation.

use sea_orm::ConnectionTrait;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[sea_orm_migration::async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(
                r#"
                ALTER TABLE manga_pipeline_artifact
                    DROP CONSTRAINT IF EXISTS ck_manga_pipeline_artifact_stage_scope
            "#,
            )
            .await?;
        manager
            .get_connection()
            .execute_unprepared(r#"
                WITH grouped AS (
                    SELECT project_id, chapter_id,
                        jsonb_object_agg(stage, payload::jsonb) AS legacy_payloads,
                        min(created_at) AS created_at, max(updated_at) AS updated_at
                    FROM manga_pipeline_artifact
                    WHERE stage IN ('character_binding', 'speech_selection', 'visual_anchor_extraction', 'item_extraction', 'comic_synthesis')
                    GROUP BY project_id, chapter_id
                )
                INSERT INTO manga_pipeline_artifact (project_id, chapter_id, stage, page_number, input_fingerprint, output_fingerprint, payload, stale, created_at, updated_at)
                SELECT project_id, chapter_id, 'preprocessing', 0,
                    'legacy:manga_preprocessing_v2:' || project_id || ':' || chapter_id,
                    md5((jsonb_build_object('project_id', project_id, 'chapter_id', chapter_id, 'created_at', created_at))::text),
                    jsonb_build_object(
                        'schema_version', 'manga_preprocessing_v2',
                        'source_events', jsonb_build_object('events', jsonb_build_array()),
                        'characters', COALESCE(legacy_payloads -> 'character_binding', '{}'::jsonb),
                        'speech', COALESCE(legacy_payloads -> 'speech_selection', jsonb_build_object('dialogues', jsonb_build_array(), 'narrations', jsonb_build_array())),
                        'visual_anchors', COALESCE(legacy_payloads -> 'visual_anchor_extraction', jsonb_build_object('scenes', jsonb_build_array())),
                        'items', COALESCE(legacy_payloads -> 'item_extraction', jsonb_build_object('items', jsonb_build_array())),
                        'comic_synthesis', COALESCE(legacy_payloads -> 'comic_synthesis', jsonb_build_object())
                    ),
                    TRUE, created_at, updated_at
                FROM grouped
                ON CONFLICT (project_id, chapter_id, stage, page_number) DO NOTHING;

                DELETE FROM manga_pipeline_artifact
                WHERE stage IN ('character_binding', 'speech_selection', 'visual_anchor_extraction', 'item_extraction', 'comic_synthesis')
            "#)
            .await?;
        manager
            .get_connection()
            .execute_unprepared(r#"
                ALTER TABLE manga_pipeline_artifact
                    ADD CONSTRAINT ck_manga_pipeline_artifact_stage_scope
                    CHECK (
                        (stage IN ('page_content', 'adaptation', 'storyboard', 'reference_discovery', 'reference_bindings', 'drawing_prompt', 'image') AND page_number > 0)
                        OR (stage IN ('preprocessing', 'page_slots') AND page_number = 0)
                    )
            "#)
            .await
            .map(|_| ())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Err(DbErr::Custom(
            "manga_preprocessing 不可逆：预处理产物可能已成为改编计划输入".into(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use sea_orm::{ConnectionTrait, DbBackend, Statement};

    #[tokio::test]
    async fn final_stage_scope_excludes_retired_preprocessing_stages() {
        let db = crate::test_support::connect_test_database().await;
        let row = db
            .query_one(Statement::from_string(
                DbBackend::Postgres,
                r#"
                    SELECT pg_get_constraintdef(oid) AS definition
                    FROM pg_constraint
                    WHERE conrelid = 'manga_pipeline_artifact'::regclass
                      AND conname = 'ck_manga_pipeline_artifact_stage_scope'
                "#
                .to_string(),
            ))
            .await
            .expect("stage constraint must be queryable")
            .expect("stage constraint must exist");
        let definition = row
            .try_get::<String>("", "definition")
            .expect("stage constraint definition must be text");

        assert!(definition.contains("'preprocessing'"));
        assert!(definition.contains("'page_slots'"));
        for retired_stage in [
            "character_binding",
            "speech_selection",
            "visual_anchor_extraction",
            "item_extraction",
            "comic_synthesis",
        ] {
            assert!(!definition.contains(retired_stage));
        }
    }
}
