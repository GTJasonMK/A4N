use a4n_db_models::models::task::task_queue;
use a4n_queue::priority::{TaskKind, TaskPriority};
use sea_orm::EntityTrait;
use tokio_util::sync::CancellationToken;

use super::super::PersistentTaskQueue;
use super::super::test_support::setup_db;

#[tokio::test]
async fn completed_task_preserves_memory_and_database_terminal_status() {
    let db = setup_db().await;
    let queue = PersistentTaskQueue::new(db.clone()).await.unwrap();
    let task_id = queue
        .submit_internal(TaskKind::LlmCall, TaskPriority::High, Some(9), None, None)
        .await
        .unwrap();
    let permit = queue
        .acquire_task_with_cancel(&task_id, CancellationToken::new())
        .await
        .unwrap()
        .unwrap();
    permit.complete();
    queue
        .update_status(&task_id, "completed", None)
        .await
        .unwrap();

    queue.cancel(&task_id).await.unwrap();

    drop(queue);
    let queue = PersistentTaskQueue::new(db.clone()).await.unwrap();
    queue
        .cancel(&task_id)
        .await
        .expect("取消已无内存缓存的终态任务仍须幂等");

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
        .submit_internal(TaskKind::LlmCall, TaskPriority::High, Some(10), None, None)
        .await
        .unwrap();
    let permit = queue
        .acquire_task_with_cancel(&task_id, CancellationToken::new())
        .await
        .unwrap()
        .unwrap();

    queue.cancel(&task_id).await.unwrap();
    permit.complete();
    queue
        .update_status(&task_id, "completed", None)
        .await
        .unwrap();

    let row = task_queue::Entity::find_by_id(task_id)
        .one(db.as_ref())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.status, "cancelled");
    assert_eq!(row.project_id, Some(10));
}
