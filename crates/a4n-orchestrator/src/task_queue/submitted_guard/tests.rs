use a4n_core::models::task::task_queue;
use a4n_queue::priority::{TaskKind, TaskPriority, TaskQueue, TaskStatus};
use sea_orm::EntityTrait;

use super::super::PersistentTaskQueue;
use super::super::test_support::setup_db;

#[tokio::test]
async fn dropped_pending_task_is_cancelled() {
    let db = setup_db().await;
    let queue = PersistentTaskQueue::new(db.clone()).await.unwrap();
    let task_id = queue
        .submit(TaskKind::LlmCall, TaskPriority::High, Some(11))
        .await
        .unwrap();

    let guard = queue.submitted_task_guard(task_id.clone());
    drop(guard);
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        loop {
            let info = queue.status(&task_id).await.unwrap();
            if matches!(info.status, TaskStatus::Cancelled) {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("dropped task cancellation was not persisted before timeout");

    let row = task_queue::Entity::find_by_id(task_id)
        .one(db.as_ref())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.status, "cancelled");
    assert_eq!(row.project_id, Some(11));
}
