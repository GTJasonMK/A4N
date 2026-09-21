//! 重新启用已接通生产链的章节视频时间线，并只保留唯一顺序状态。

use sea_orm::{ConnectionTrait, DbBackend};
use sea_orm_migration::prelude::*;

const MIGRATION_NAME: &str = "chapter_video_timeline_contract";

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
            "{MIGRATION_NAME} 不可逆：删除会破坏已提交插入片段的稳定身份"
        )))
    }
}

const UP_SQL: &str = r#"
CREATE TABLE chapter_video_timeline (
    id BIGSERIAL PRIMARY KEY,
    project_id BIGINT NOT NULL REFERENCES novel_project(id) ON DELETE CASCADE,
    chapter_id BIGINT NOT NULL REFERENCES chapter(id) ON DELETE CASCADE,
    workflow_mode TEXT NOT NULL,
    revision BIGINT NOT NULL DEFAULT 1,
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP NOT NULL DEFAULT NOW(),
    CONSTRAINT chapter_video_timeline_scope_unique
        UNIQUE (project_id, chapter_id, workflow_mode),
    CONSTRAINT chapter_video_timeline_workflow_mode_nonempty
        CHECK (NULLIF(BTRIM(workflow_mode), '') IS NOT NULL),
    CONSTRAINT chapter_video_timeline_revision_positive CHECK (revision > 0)
);

CREATE TABLE chapter_video_timeline_item (
    id BIGSERIAL PRIMARY KEY,
    timeline_id BIGINT NOT NULL REFERENCES chapter_video_timeline(id) ON DELETE CASCADE,
    position INTEGER NULL,
    item_kind TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'active',
    native_source_shot_id TEXT NULL,
    native_segment_index_in_shot INTEGER NULL,
    insert_plan_prompt_id BIGINT NULL REFERENCES chapter_video_prompt(id) ON DELETE RESTRICT,
    workflow_run_id BIGINT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP NOT NULL DEFAULT NOW(),
    CONSTRAINT chapter_video_timeline_item_position_unique
        UNIQUE (timeline_id, position) DEFERRABLE INITIALLY DEFERRED,
    CONSTRAINT chapter_video_timeline_item_position_positive
        CHECK (position IS NULL OR position > 0),
    CONSTRAINT chapter_video_timeline_item_kind_valid
        CHECK (item_kind IN ('native', 'inserted')),
    CONSTRAINT chapter_video_timeline_item_status_valid
        CHECK (status IN ('active', 'deleted', 'needs_reanchor')),
    CONSTRAINT chapter_video_timeline_item_position_status_consistent
        CHECK ((status = 'deleted') = (position IS NULL)),
    CONSTRAINT chapter_video_timeline_item_identity_valid CHECK (
        (item_kind = 'native'
            AND NULLIF(BTRIM(native_source_shot_id), '') IS NOT NULL
            AND native_segment_index_in_shot > 0
            AND insert_plan_prompt_id IS NULL
            AND workflow_run_id IS NULL)
        OR
        (item_kind = 'inserted'
            AND native_source_shot_id IS NULL
            AND native_segment_index_in_shot IS NULL
            AND insert_plan_prompt_id IS NOT NULL
            AND workflow_run_id > 0)
    ),
    CONSTRAINT chapter_video_timeline_item_reanchor_kind_valid
        CHECK (status <> 'needs_reanchor' OR item_kind = 'inserted')
);

CREATE UNIQUE INDEX uidx_chapter_video_timeline_native_identity
    ON chapter_video_timeline_item (
        timeline_id,
        native_source_shot_id,
        native_segment_index_in_shot
    )
    WHERE item_kind = 'native';
CREATE UNIQUE INDEX uidx_chapter_video_timeline_insert_prompt
    ON chapter_video_timeline_item (insert_plan_prompt_id)
    WHERE item_kind = 'inserted';
CREATE UNIQUE INDEX uidx_chapter_video_timeline_workflow_run
    ON chapter_video_timeline_item (workflow_run_id)
    WHERE item_kind = 'inserted';

ALTER TABLE chapter_video_prompt
    ADD COLUMN timeline_item_id BIGINT NULL
        REFERENCES chapter_video_timeline_item(id) ON DELETE SET NULL,
    ADD CONSTRAINT chapter_video_prompt_timeline_identity_consistent CHECK ((
        timeline_item_id IS NULL
        OR (
            metadata_json IS NOT NULL
            AND metadata_json::jsonb ->> 'source_prompt_type' = 'inserted_segment'
            AND jsonb_typeof(metadata_json::jsonb -> 'segment_recipe') = 'object'
        )
    ) IS TRUE);
CREATE UNIQUE INDEX uidx_chapter_video_prompt_timeline_item
    ON chapter_video_prompt (timeline_item_id)
    WHERE timeline_item_id IS NOT NULL;

ALTER TABLE generated_video
    ADD COLUMN timeline_item_id BIGINT NULL
        REFERENCES chapter_video_timeline_item(id) ON DELETE SET NULL,
    ADD CONSTRAINT generated_video_timeline_identity_consistent CHECK ((
        timeline_item_id IS NULL
        OR (
            metadata_json IS NOT NULL
            AND metadata_json::jsonb ->> 'source_prompt_type' = 'inserted_segment'
        )
    ) IS TRUE);
CREATE INDEX idx_generated_video_timeline_item
    ON generated_video (timeline_item_id);
"#;

#[cfg(test)]
mod tests {
    use super::{Migration, UP_SQL};
    use sea_orm::{ConnectionTrait, Statement};
    use sea_orm_migration::prelude::{MigrationName, MigrationTrait, SchemaManager};

    #[test]
    fn contract_uses_one_position_source_and_no_mirrors() {
        assert_eq!(Migration.name(), "chapter_video_timeline_contract");
        for removed in [
            "head_item_id",
            "tail_item_id",
            "previous_item_id",
            "next_item_id",
            "provenance_json",
            "deleted_at",
            "chapter_video_timeline_event",
        ] {
            assert!(
                !UP_SQL.contains(removed),
                "unexpected duplicate state: {removed}"
            );
        }
        assert!(UP_SQL.contains("position INTEGER NULL"));
        assert!(UP_SQL.contains("workflow_run_id BIGINT NULL"));
    }

    #[tokio::test]
    async fn final_schema_has_one_timeline_state_source() {
        let db = crate::test_support::connect_test_database().await;
        let row = db
            .query_one(Statement::from_string(
                db.get_database_backend(),
                "SELECT to_regclass('chapter_video_timeline') IS NOT NULL AS timeline_exists, \
                        EXISTS (SELECT 1 FROM information_schema.columns \
                            WHERE table_schema = current_schema() \
                              AND table_name = 'chapter_video_timeline_item' \
                              AND column_name = 'position') AS position_exists, \
                        NOT EXISTS (SELECT 1 FROM information_schema.columns \
                            WHERE table_schema = current_schema() \
                              AND table_name IN ('chapter_video_timeline', 'chapter_video_timeline_item') \
                              AND column_name IN ( \
                                  'head_item_id', 'tail_item_id', 'previous_item_id', \
                                  'next_item_id', 'provenance_json', 'deleted_at' \
                              )) AS mirrors_removed"
                    .to_string(),
            ))
            .await
            .expect("查询时间线最终 schema")
            .expect("时间线最终 schema 查询应返回一行");

        assert!(row.try_get::<bool>("", "timeline_exists").unwrap());
        assert!(row.try_get::<bool>("", "position_exists").unwrap());
        assert!(row.try_get::<bool>("", "mirrors_removed").unwrap());

        let manager = SchemaManager::new(&db);
        let error = Migration
            .down(&manager)
            .await
            .expect_err("migration is irreversible");
        assert!(error.to_string().contains("不可逆"));
    }
}
