use a4n_core::models::task::task_queue;
use a4n_queue::priority::{TaskKind, TaskPriority, TaskQueue, TaskStatus};
use sea_orm::EntityTrait;

use super::super::super::PersistentTaskQueue;
use super::super::super::test_support::setup_db;

#[tokio::test]
async fn completed_task_preserves_memory_and_database_terminal_status() {
    let db = setup_db().await;
    let queue = PersistentTaskQueue::new(db.clone()).await.unwrap();
    let task_id = queue
        .submit(TaskKind::LlmCall, TaskPriority::High, Some(9))
        .await
        .unwrap();
    let permit = queue.acquire(TaskKind::LlmCall).await.unwrap();
    assert_eq!(permit.task_id(), &task_id);
    permit.complete();
    queue.mark_done(&task_id).await.unwrap();

    queue.cancel(&task_id).await.unwrap();

    let info = queue.status(&task_id).await.unwrap();
    assert!(matches!(info.status, TaskStatus::Completed));
    let row = task_queue::Entity::find_by_id(task_id)
        .one(db.as_ref())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.status, "completed");
    assert_eq!(row.project_id, Some(9));
}

#[tokio::test]
async fn mark_done_after_running_cancel_preserves_cancelled_status() {
    let db = setup_db().await;
    let queue = PersistentTaskQueue::new(db.clone()).await.unwrap();
    let task_id = queue
        .submit(TaskKind::LlmCall, TaskPriority::High, Some(10))
        .await
        .unwrap();
    let permit = queue.acquire(TaskKind::LlmCall).await.unwrap();
    assert_eq!(permit.task_id(), &task_id);

    queue.cancel(&task_id).await.unwrap();
    permit.complete();
    queue.mark_done(&task_id).await.unwrap();

    let info = queue.status(&task_id).await.unwrap();
    assert!(matches!(info.status, TaskStatus::Cancelled));
    let row = task_queue::Entity::find_by_id(task_id)
        .one(db.as_ref())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.status, "cancelled");
    assert_eq!(row.project_id, Some(10));
}
