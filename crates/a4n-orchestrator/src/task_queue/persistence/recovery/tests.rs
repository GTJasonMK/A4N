use a4n_core::models::task::task_queue;
use sea_orm::EntityTrait;

use super::super::super::PersistentTaskQueue;
use super::super::super::test_support::{insert_task_row, setup_db};

#[tokio::test]
async fn interrupts_standalone_active_tasks() {
    let db = setup_db().await;
    insert_task_row(db.as_ref(), "recover-running", "running", None).await;
    insert_task_row(db.as_ref(), "recover-pending", "pending", None).await;

    let _queue = PersistentTaskQueue::new(db.clone()).await.unwrap();

    for task_id in ["recover-running", "recover-pending"] {
        let row = task_queue::Entity::find_by_id(task_id.to_string())
            .one(db.as_ref())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(row.status, "interrupted", "{task_id} should be interrupted");
        assert!(row.completed_at.is_some());
    }
}

#[tokio::test]
async fn interrupts_historical_dag_tasks() {
    let db = setup_db().await;
    insert_task_row(db.as_ref(), "dag-child", "pending", Some("historical-dag")).await;

    let _queue = PersistentTaskQueue::new(db.clone()).await.unwrap();

    let row = task_queue::Entity::find_by_id("dag-child".to_string())
        .one(db.as_ref())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.status, "interrupted");
    assert_eq!(row.dag_id.as_deref(), Some("historical-dag"));
    assert!(row.completed_at.is_some());
}
