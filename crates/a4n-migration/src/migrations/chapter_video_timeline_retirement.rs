//! 退役未接通的章节视频插入时间线；成片插入只保留 assembly_insert。

use sea_orm::{ConnectionTrait, DbBackend};
use sea_orm_migration::prelude::*;

const MIGRATION_NAME: &str = "chapter_video_timeline_retirement";

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
            "{MIGRATION_NAME} 不可逆：章节视频插入时间线已由 assembly_insert 取代"
        )))
    }
}

const UP_SQL: &str = r#"
DO $$
BEGIN
    IF EXISTS (
        SELECT 1
        FROM chapter_video_timeline_item
        WHERE item_kind = 'inserted' OR insert_plan_prompt_id IS NOT NULL
    ) OR EXISTS (
        SELECT 1 FROM chapter_video_prompt WHERE timeline_item_id IS NOT NULL
    ) OR EXISTS (
        SELECT 1 FROM generated_video WHERE timeline_item_id IS NOT NULL
    ) THEN
        RAISE EXCEPTION
            'chapter_video_timeline_retirement: committed inserted timeline data requires explicit resolution';
    END IF;
END $$;

CREATE TEMP TABLE _a4n_retired_video_insert_workflow ON COMMIT DROP AS
SELECT id
FROM workflow_run
WHERE workflow_kind = 'video_inserted_segment';

CREATE TEMP TABLE _a4n_retired_video_insert_task ON COMMIT DROP AS
SELECT id
FROM task_queue
WHERE workflow_run_id IN (SELECT id FROM _a4n_retired_video_insert_workflow);

CREATE TEMP TABLE _a4n_retired_video_insert_generation_run ON COMMIT DROP AS
SELECT id
FROM generation_run
WHERE workflow_run_id IN (SELECT id FROM _a4n_retired_video_insert_workflow);

DELETE FROM llm_call_trace
WHERE workflow_run_id IN (SELECT id FROM _a4n_retired_video_insert_workflow)
   OR generation_run_id IN (SELECT id FROM _a4n_retired_video_insert_generation_run);

DELETE FROM generation_run_target
WHERE generation_run_id IN (SELECT id FROM _a4n_retired_video_insert_generation_run);

DELETE FROM generation_start_request
WHERE task_id IN (SELECT id FROM _a4n_retired_video_insert_task);

DELETE FROM task_dependency
WHERE task_id IN (SELECT id FROM _a4n_retired_video_insert_task)
   OR depends_on_task_id IN (SELECT id FROM _a4n_retired_video_insert_task);

DELETE FROM task_queue
WHERE id IN (SELECT id FROM _a4n_retired_video_insert_task);

DELETE FROM generation_run
WHERE id IN (SELECT id FROM _a4n_retired_video_insert_generation_run);

DELETE FROM workflow_progress_event
WHERE workflow_run_id IN (SELECT id FROM _a4n_retired_video_insert_workflow);

DELETE FROM workflow_checkpoint
WHERE workflow_run_id IN (SELECT id FROM _a4n_retired_video_insert_workflow);

DELETE FROM workflow_run
WHERE id IN (SELECT id FROM _a4n_retired_video_insert_workflow);

ALTER TABLE chapter_video_prompt DROP COLUMN IF EXISTS timeline_item_id;
ALTER TABLE generated_video DROP COLUMN IF EXISTS timeline_item_id;

DROP TABLE chapter_video_timeline_event;
DROP TABLE chapter_video_timeline_item;
DROP TABLE chapter_video_timeline;
"#;

#[cfg(test)]
mod tests {
    use super::{Migration, UP_SQL};
    use sea_orm::{ConnectionTrait, DbBackend, Statement};
    use sea_orm_migration::prelude::{MigrationTrait, SchemaManager};

    #[test]
    fn retirement_rejects_committed_insertions_and_removes_the_whole_schema() {
        assert!(UP_SQL.contains("committed inserted timeline data requires explicit resolution"));
        assert!(UP_SQL.contains("workflow_kind = 'video_inserted_segment'"));
        assert_eq!(
            UP_SQL
                .matches("DROP COLUMN IF EXISTS timeline_item_id")
                .count(),
            2
        );
        assert_eq!(
            UP_SQL.matches("DROP TABLE chapter_video_timeline").count(),
            3
        );
        assert!(
            UP_SQL.find("DELETE FROM generation_start_request").unwrap()
                < UP_SQL.find("DELETE FROM task_queue").unwrap()
        );
    }

    #[tokio::test]
    async fn retirement_removes_generation_start_request_before_its_task() {
        let db = crate::test_support::connect_test_database().await;
        crate::migrations::chapter_video_timeline::Migration
            .up(&SchemaManager::new(&db))
            .await
            .unwrap();
        db.execute_unprepared(
            r#"
INSERT INTO workflow_run (
    business_key, workflow_kind, status, can_resume, created_at, updated_at
) VALUES (
    'retired-video-insert-test', 'video_inserted_segment', 'pending', FALSE, NOW(), NOW()
);
INSERT INTO task_queue (
    id, task_type, priority, status, workflow_run_id, created_at
) VALUES (
    '11111111-1111-4111-8111-111111111111',
    'video_inserted_segment',
    'medium',
    'pending',
    (SELECT id FROM workflow_run WHERE business_key = 'retired-video-insert-test'),
    NOW()
);
INSERT INTO generation_start_request (
    requested_by_user_id,
    start_request_id,
    task_id,
    input_fingerprint_sha256,
    created_at,
    last_seen_at
) VALUES (
    1,
    '22222222-2222-4222-8222-222222222222',
    '11111111-1111-4111-8111-111111111111',
    repeat('0', 64),
    NOW(),
    NOW()
);
"#,
        )
        .await
        .unwrap();

        Migration.up(&SchemaManager::new(&db)).await.unwrap();

        let remaining = db
            .query_one(Statement::from_string(
                DbBackend::Postgres,
                r#"
SELECT
    (SELECT COUNT(*) FROM generation_start_request
     WHERE task_id = '11111111-1111-4111-8111-111111111111')
  + (SELECT COUNT(*) FROM task_queue
     WHERE id = '11111111-1111-4111-8111-111111111111') AS count
"#,
            ))
            .await
            .unwrap()
            .unwrap()
            .try_get::<i64>("", "count")
            .unwrap();
        assert_eq!(remaining, 0);
    }
}
