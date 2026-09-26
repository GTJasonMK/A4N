use super::*;
use sea_orm::{DbBackend, Statement, TransactionTrait};

#[tokio::test]
async fn deleting_local_targets_retires_comparisons_and_preserves_decisions() {
    let db = crate::test_support::connect_test_database().await;
    let tx = db.begin().await.unwrap();
    tx.execute_unprepared(r#"
        CREATE TEMP TABLE generation_spec (id bigint) ON COMMIT DROP;
        CREATE TEMP TABLE galgame_scene (id bigint) ON COMMIT DROP;
        CREATE TEMP TABLE galgame_line (id bigint) ON COMMIT DROP;
        CREATE TEMP TABLE prompt_artifact (id bigint) ON COMMIT DROP;
        CREATE TEMP TABLE media_asset (id bigint) ON COMMIT DROP;
        CREATE TEMP TABLE video_segment (id bigint) ON COMMIT DROP;
        CREATE TEMP TABLE source_sync_conflict (
            id bigint, conflict_scope text, target_kind text, target_id bigint, status text,
            resolution_strategy text, local_summary_json json, affected_refs_json json,
            decision_json json, resolved_at timestamp
        ) ON COMMIT DROP;
        INSERT INTO generation_spec VALUES (10),(11);
        INSERT INTO galgame_line VALUES (10);
        INSERT INTO source_sync_conflict VALUES
            (1,'generation_spec','segment',10,'pending',NULL,'{"generation_spec_id":10}','{}',NULL,NULL),
            (2,'profile','character',8,'pending',NULL,'{"generation_spec_id":11}','{}',NULL,NULL),
            (3,'generation_spec','segment',10,'ignored','keep_local','{"generation_spec_id":10}','{}','{"user_action":"keep_local","note":"original"}','2026-01-01'),
            (4,'text_anchor','galgame_line',10,'pending',NULL,'{"galgame_line_id":10}','{}',NULL,NULL),
            (5,'generation_spec','segment',12,'pending',NULL,'{"generation_spec_id":12}','{}',NULL,NULL);
    "#).await.unwrap();
    tx.execute_unprepared(&LOCAL_REFERENCES_SQL.replacen("CREATE VIEW", "CREATE TEMP VIEW", 1))
        .await
        .unwrap();
    for table in ["generation_spec", "galgame_line"] {
        tx.execute_unprepared(&format!(
            "CREATE TRIGGER test_retirement AFTER DELETE ON {table} REFERENCING OLD TABLE AS deleted_local_targets FOR EACH STATEMENT EXECUTE FUNCTION a4n_retire_source_sync_deleted_targets()"
        )).await.unwrap();
    }
    tx.execute_unprepared("DISCARD PLANS; DELETE FROM generation_spec WHERE id IN (10,11);")
        .await
        .unwrap();
    let rows = tx.query_all(Statement::from_string(DbBackend::Postgres,
        "SELECT id,status,resolution_strategy,decision_json FROM source_sync_conflict ORDER BY id"
    )).await.unwrap();
    let statuses = rows
        .iter()
        .map(|row| row.try_get::<String>("", "status").unwrap())
        .collect::<Vec<_>>();
    assert_eq!(
        statuses,
        ["obsolete", "obsolete", "obsolete", "pending", "pending"]
    );
    assert_eq!(
        rows[2]
            .try_get::<serde_json::Value>("", "decision_json")
            .unwrap(),
        serde_json::json!({"user_action":"keep_local","note":"original"})
    );
    tx.execute_unprepared(r#"
        UPDATE source_sync_conflict SET local_summary_json = '{"generation_spec_id":"historical opaque value"}' WHERE id = 1;
        DELETE FROM generation_spec WHERE id = 999;
        DELETE FROM galgame_line WHERE id = 10;
    "#).await.unwrap();
    tx.execute_unprepared(RETIRE_EXISTING_SQL).await.unwrap();
    let count = tx
        .query_one(Statement::from_string(
            DbBackend::Postgres,
            "SELECT count(*) AS count FROM source_sync_conflict WHERE status = 'obsolete'",
        ))
        .await
        .unwrap()
        .unwrap()
        .try_get::<i64>("", "count")
        .unwrap();
    assert_eq!(count, 5);
    tx.rollback().await.unwrap();
}
