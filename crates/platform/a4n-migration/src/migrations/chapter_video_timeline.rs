//! 章节视频稳定时间线与插入片段身份。

use sea_orm::{ConnectionTrait, DbBackend};
use sea_orm_migration::prelude::*;

const MIGRATION_NAME: &str = "chapter_video_timeline";

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
CREATE TABLE IF NOT EXISTS chapter_video_timeline (
    id BIGSERIAL PRIMARY KEY,
    project_id BIGINT NOT NULL REFERENCES novel_project(id) ON DELETE CASCADE,
    chapter_id BIGINT NOT NULL REFERENCES chapter(id) ON DELETE CASCADE,
    workflow_mode TEXT NOT NULL,
    revision BIGINT NOT NULL DEFAULT 1,
    head_item_id BIGINT NULL,
    tail_item_id BIGINT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP NOT NULL DEFAULT NOW(),
    CONSTRAINT chapter_video_timeline_revision_positive CHECK (revision > 0),
    CONSTRAINT chapter_video_timeline_scope_unique UNIQUE (project_id, chapter_id, workflow_mode)
);

CREATE TABLE IF NOT EXISTS chapter_video_timeline_item (
    id BIGSERIAL PRIMARY KEY,
    timeline_id BIGINT NOT NULL REFERENCES chapter_video_timeline(id) ON DELETE CASCADE,
    item_kind TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'active',
    native_source_shot_id TEXT NULL,
    native_segment_index_in_shot INTEGER NULL,
    native_segment_number INTEGER NULL,
    insert_plan_prompt_id BIGINT NULL REFERENCES chapter_video_prompt(id) ON DELETE RESTRICT,
    previous_item_id BIGINT NULL REFERENCES chapter_video_timeline_item(id) ON DELETE SET NULL,
    next_item_id BIGINT NULL REFERENCES chapter_video_timeline_item(id) ON DELETE SET NULL,
    provenance_json JSONB NULL,
    deleted_at TIMESTAMP NULL,
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP NOT NULL DEFAULT NOW(),
    CONSTRAINT chapter_video_timeline_item_kind_valid CHECK (item_kind IN ('native', 'inserted')),
    CONSTRAINT chapter_video_timeline_item_status_valid CHECK (status IN ('active', 'deleted', 'needs_reanchor')),
    CONSTRAINT chapter_video_timeline_item_identity_valid CHECK (
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
    CONSTRAINT chapter_video_timeline_item_reanchor_kind_valid CHECK (
        status <> 'needs_reanchor' OR item_kind = 'inserted'
    ),
    CONSTRAINT chapter_video_timeline_item_deleted_at_valid CHECK (
        (status = 'deleted') = (deleted_at IS NOT NULL)
    )
);

CREATE UNIQUE INDEX IF NOT EXISTS uidx_chapter_video_timeline_native_identity
    ON chapter_video_timeline_item (timeline_id, native_source_shot_id, native_segment_index_in_shot)
    WHERE item_kind = 'native';
CREATE INDEX IF NOT EXISTS idx_chapter_video_timeline_item_previous
    ON chapter_video_timeline_item (timeline_id, previous_item_id);
CREATE INDEX IF NOT EXISTS idx_chapter_video_timeline_item_next
    ON chapter_video_timeline_item (timeline_id, next_item_id);
CREATE UNIQUE INDEX IF NOT EXISTS uidx_chapter_video_timeline_inserted_workflow_run
    ON chapter_video_timeline_item (
        timeline_id,
        (provenance_json ->> 'workflow_run_id')
    )
    WHERE item_kind = 'inserted'
      AND provenance_json ->> 'workflow_run_id' IS NOT NULL;
CREATE UNIQUE INDEX IF NOT EXISTS uidx_chapter_video_timeline_insert_prompt
    ON chapter_video_timeline_item (insert_plan_prompt_id)
    WHERE item_kind = 'inserted';

CREATE TABLE IF NOT EXISTS chapter_video_timeline_event (
    id BIGSERIAL PRIMARY KEY,
    timeline_id BIGINT NOT NULL REFERENCES chapter_video_timeline(id) ON DELETE CASCADE,
    revision BIGINT NOT NULL,
    operation TEXT NOT NULL,
    item_id BIGINT NULL REFERENCES chapter_video_timeline_item(id) ON DELETE SET NULL,
    payload_json JSONB NOT NULL DEFAULT '{}'::jsonb,
    actor_user_id BIGINT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_chapter_video_timeline_event_scope
    ON chapter_video_timeline_event (timeline_id, revision DESC, id DESC);
CREATE UNIQUE INDEX IF NOT EXISTS uidx_chapter_video_timeline_event_revision
    ON chapter_video_timeline_event (timeline_id, revision);

ALTER TABLE chapter_video_prompt
    ADD COLUMN IF NOT EXISTS timeline_item_id BIGINT NULL
        REFERENCES chapter_video_timeline_item(id) ON DELETE SET NULL;
ALTER TABLE generated_video
    ADD COLUMN IF NOT EXISTS timeline_item_id BIGINT NULL
        REFERENCES chapter_video_timeline_item(id) ON DELETE SET NULL;
CREATE INDEX IF NOT EXISTS idx_chapter_video_prompt_timeline_item
    ON chapter_video_prompt (timeline_item_id);
CREATE UNIQUE INDEX IF NOT EXISTS uidx_chapter_video_prompt_timeline_item
    ON chapter_video_prompt (timeline_item_id)
    WHERE timeline_item_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_generated_video_timeline_item
    ON generated_video (timeline_item_id);
"#,
            )
            .await?;
        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Err(DbErr::Custom(
            "chapter_video_timeline 不可逆：删除会破坏已提交插入片段的稳定身份".to_string(),
        ))
    }
}
