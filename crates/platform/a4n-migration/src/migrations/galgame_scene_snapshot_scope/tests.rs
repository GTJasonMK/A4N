use sea_orm::{ConnectionTrait, DbBackend, Statement, TransactionTrait};

#[tokio::test]
async fn snapshot_scope_includes_used_scenes_but_not_unrelated_history() {
    let db = crate::test_support::connect_test_database().await;
    let tx = db.begin().await.unwrap();
    tx.execute_unprepared(r#"
        CREATE TEMP TABLE novel_project (id bigint, project_kind text, root_project_id bigint, source_snapshot_id bigint) ON COMMIT DROP;
        CREATE TEMP TABLE project_source_snapshot (id bigint, root_project_id bigint) ON COMMIT DROP;
        CREATE TEMP TABLE galgame_script (id bigint, project_id bigint) ON COMMIT DROP;
        CREATE TEMP TABLE galgame_scene (script_id bigint, source_snapshot_chapter_id bigint) ON COMMIT DROP;
        CREATE TEMP TABLE source_snapshot_chapter (id bigint, snapshot_id bigint) ON COMMIT DROP;
        INSERT INTO novel_project VALUES (1,'text',NULL,NULL),(2,'galgame',1,20),(3,'video',1,20),(4,'galgame',1,20);
        INSERT INTO project_source_snapshot VALUES (10,1),(20,1),(30,1),(40,9);
        INSERT INTO galgame_script VALUES (1,2),(2,4);
        INSERT INTO galgame_scene VALUES (1,11),(2,31);
        INSERT INTO source_snapshot_chapter VALUES (11,10),(31,30);
        DISCARD PLANS;
    "#).await.unwrap();
    for (project, snapshot, expected) in [
        (2, 10, true),
        (2, 20, true),
        (2, 30, false),
        (2, 40, false),
        (3, 10, false),
        (3, 20, true),
        (4, 30, true),
        (1, 10, true),
        (1, 40, false),
    ] {
        let row = tx
            .query_one(Statement::from_sql_and_values(
                DbBackend::Postgres,
                "SELECT a4n_project_source_snapshot_allowed($1, $2) AS allowed",
                [i64::from(project).into(), i64::from(snapshot).into()],
            ))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            row.try_get::<bool>("", "allowed").unwrap(),
            expected,
            "project={project}, snapshot={snapshot}"
        );
    }
    tx.rollback().await.unwrap();
}
