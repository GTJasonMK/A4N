use sea_orm::{ConnectionTrait, DbBackend, Statement};

#[tokio::test]
async fn global_task_history_created_index_is_installed_by_schema_migrations() {
    let db = a4n_migration::test_support::connect_test_database().await;
    let row = db
        .query_one(Statement::from_string(
            DbBackend::Postgres,
            "SELECT indexdef FROM pg_indexes WHERE schemaname = current_schema() AND indexname = 'idx_task_queue_created'".to_string(),
        ))
        .await
        .unwrap()
        .expect("global task history created index must exist");
    let indexdef: String = row.try_get("", "indexdef").unwrap();

    assert!(indexdef.contains("task_queue"));
    assert!(indexdef.contains("created_at DESC"));
}
