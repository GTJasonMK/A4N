use a4n_core::models::task::task_queue;
use a4n_queue::priority::{TaskKind, TaskPriority, TaskQueue};
use sea_orm::EntityTrait;

use super::super::super::PersistentTaskQueue;
use super::super::super::test_support::setup_db;

#[tokio::test]
async fn mark_done_updates_database_status() {
    let db = setup_db().await;
    let queue = PersistentTaskQueue::new(db.clone()).await.unwrap();
    let task_id = queue
        .submit(TaskKind::LlmCall, TaskPriority::High, None)
        .await
        .unwrap();

    queue.mark_done(&task_id).await.unwrap();

    let row = task_queue::Entity::find_by_id(task_id)
        .one(db.as_ref())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.status, "completed");
    assert!(row.completed_at.is_some());
}

#[tokio::test]
async fn mark_failed_records_error() {
    let db = setup_db().await;
    let queue = PersistentTaskQueue::new(db.clone()).await.unwrap();
    let task_id = queue
        .submit(TaskKind::LlmCall, TaskPriority::Medium, None)
        .await
        .unwrap();

    queue.mark_failed(&task_id, "LLM 调用超时").await.unwrap();

    let row = task_queue::Entity::find_by_id(task_id)
        .one(db.as_ref())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.status, "failed");
    assert_eq!(row.error_message.as_deref(), Some("LLM 调用超时"));
}
