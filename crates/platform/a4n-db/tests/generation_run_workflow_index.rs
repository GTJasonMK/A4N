use sea_orm::{ConnectionTrait, DbBackend, Statement};

#[tokio::test]
async fn workflow_generation_run_index_is_partial_and_installed_by_schema_migrations() {
    let db = a4n_migration::test_support::connect_test_database().await;
    let row = db
        .query_one(Statement::from_string(
            DbBackend::Postgres,
            "SELECT indexdef FROM pg_indexes WHERE schemaname = current_schema() AND indexname = 'idx_generation_run_workflow_created'".to_string(),
        ))
        .await
        .unwrap()
        .expect("workflow generation run index must exist");
    let indexdef: String = row.try_get("", "indexdef").unwrap();

    assert!(indexdef.contains("generation_run"));
    assert!(indexdef.contains("workflow_run_id, created_at"));
    assert!(indexdef.contains("WHERE (workflow_run_id IS NOT NULL)"));
}
