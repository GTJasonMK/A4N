use sea_orm::{ConnectionTrait, DbBackend, Statement, TransactionTrait};

#[tokio::test]
async fn legacy_scene_sources_require_one_complete_unchanged_frozen_cohort() {
    let db = crate::test_support::connect_test_database().await;
    let tx = db.begin().await.unwrap();
    tx.execute_unprepared(r#"
        CREATE TEMP TABLE galgame_script (id bigint, project_id bigint) ON COMMIT DROP;
        CREATE TEMP TABLE galgame_scene (id bigint, script_id bigint, source_snapshot_chapter_id bigint,
            scene_index int, entry_kind text, shooting_scene_identity text) ON COMMIT DROP;
        CREATE TEMP TABLE source_snapshot_chapter (id bigint, snapshot_id bigint, source_chapter_id bigint,
            visual_plan_id bigint, visual_plan_version int, visual_plan_sha256 text, content_hash text) ON COMMIT DROP;
        CREATE TEMP TABLE chapter_shooting_script (source_snapshot_chapter_id bigint, visual_plan_id bigint,
            visual_plan_version int, visual_plan_sha256 text, source_content_sha256 text, script_json jsonb) ON COMMIT DROP;
        CREATE TEMP TABLE source_sync_conflict (derived_project_id bigint, status text, resolution_strategy text) ON COMMIT DROP;
        INSERT INTO galgame_script VALUES (1,1),(2,2),(3,3),(4,4),(5,5),(6,6);
        INSERT INTO source_snapshot_chapter VALUES
            (11,1,1,1,1,'plan','body'),(21,2,2,2,1,'plan','body'),
            (31,3,3,3,1,'plan','body'),(32,4,3,4,1,'plan','body'),
            (41,5,4,5,1,'plan','body'),(51,6,5,6,1,'plan','body'),(61,7,6,7,1,'plan','body');
        INSERT INTO chapter_shooting_script VALUES
            (11,1,1,'plan','body','{"scenes":[{"scene_index":1,"scene_identity":"unit:first"},{"scene_index":2,"scene_identity":"unit:second"}]}'),
            (21,2,1,'plan','body','{"scenes":[{"scene_index":1,"scene_identity":"unit:first"},{"scene_index":2,"scene_identity":"unit:second"}]}'),
            (31,3,1,'plan','body','{"scenes":[{"scene_index":1,"scene_identity":"unit:first"},{"scene_index":2,"scene_identity":"unit:second"}]}'),
            (32,4,1,'plan','body','{"scenes":[{"scene_index":1,"scene_identity":"unit:first"},{"scene_index":2,"scene_identity":"unit:second"}]}'),
            (41,5,1,'plan','body','{"scenes":[{"scene_index":1,"scene_identity":"unit:recorded"}]}'),
            (51,6,1,'plan','body','{"scenes":[{"scene_index":1,"scene_identity":"unit:first"}]}'),
            (61,7,1,'plan','body','{"scenes":[{"scene_index":1,"scene_identity":"unit:first"}]}'),
            (61,7,1,'plan','body','{"scenes":[{"scene_index":1,"scene_identity":"unit:other"}]}');
        INSERT INTO galgame_scene VALUES
            (1,1,11,4,'main',NULL),(2,1,11,9,'main',NULL),(3,1,11,10,'branch',NULL),
            (4,2,21,1,'main',NULL),(5,3,31,1,'main',NULL),(6,3,32,2,'main',NULL),
            (7,4,41,1,'main','unit:recorded'),(8,5,51,1,'main',NULL),(9,6,61,1,'main',NULL);
        INSERT INTO source_sync_conflict VALUES (5,'obsolete','regenerate_scene');
        DISCARD PLANS;
        SELECT a4n_backfill_galgame_scene_sources(1);
        SELECT a4n_backfill_galgame_scene_sources(NULL::bigint);
        SELECT a4n_backfill_galgame_scene_sources(NULL::bigint);
    "#).await.unwrap();
    let rows = tx
        .query_all(Statement::from_string(
            DbBackend::Postgres,
            "SELECT id, shooting_scene_identity FROM galgame_scene ORDER BY id",
        ))
        .await
        .unwrap();
    let actual = rows
        .into_iter()
        .map(|row| {
            (
                row.try_get::<i64>("", "id").unwrap(),
                row.try_get::<Option<String>>("", "shooting_scene_identity")
                    .unwrap(),
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(
        actual,
        vec![
            (1, Some("unit:first".into())),
            (2, Some("unit:second".into())),
            (3, None),
            (4, None),
            (5, None),
            (6, None),
            (7, Some("unit:recorded".into())),
            (8, None),
            (9, None),
        ]
    );
    tx.rollback().await.unwrap();
}
