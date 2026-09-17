use std::sync::Arc;

use a4n_db_models::models::task::task_queue;
use chrono::Utc;
use sea_orm::{DatabaseConnection, EntityTrait, Set};

pub(super) async fn setup_db() -> Arc<DatabaseConnection> {
    let db = a4n_migration::test_support::connect_test_database().await;
    let packages = a4n_api_contract::package::package_catalog()
        .packages
        .into_iter()
        .map(|package| package.id.as_str().to_string())
        .collect::<Vec<_>>();
    a4n_db::package::ensure_host_package_states(&db, &packages)
        .await
        .expect("queue tests require installed, enabled packages");
    Arc::new(db)
}

pub(super) async fn insert_task_row(
    db: &DatabaseConnection,
    id: &str,
    status: &str,
    dag_id: Option<&str>,
) {
    let now = Utc::now().naive_utc();
    task_queue::Entity::insert(task_queue::ActiveModel {
        id: Set(id.to_string()),
        task_type: Set("llm_call".to_string()),
        priority: Set("high".to_string()),
        status: Set(status.to_string()),
        project_id: Set(Some(7)),
        root_project_id: Set(None),
        source_snapshot_id: Set(None),
        task_scope: Set(None),
        workflow_run_id: Set(None),
        payload: Set(Some(serde_json::json!({"id": id}))),
        progress: Set(None),
        error_message: Set(None),
        dag_id: Set(dag_id.map(ToString::to_string)),
        created_at: Set(now),
        started_at: Set((status == "running").then_some(now)),
        completed_at: Set((status == "completed").then_some(now)),
    })
    .exec(db)
    .await
    .unwrap();
}
