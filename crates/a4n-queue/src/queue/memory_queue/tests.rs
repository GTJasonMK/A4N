use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;
use tokio_util::sync::CancellationToken;

use super::MemoryTaskQueue;
use crate::queue::priority::{TaskInfo, TaskKind, TaskPriority, TaskStatus};

fn pending_task(id: &str, priority: TaskPriority) -> TaskInfo {
    TaskInfo {
        id: id.to_string(),
        task_type: TaskKind::LlmCall,
        priority,
        status: TaskStatus::Pending,
        project_id: None,
        root_project_id: None,
        source_snapshot_id: None,
        task_scope: None,
        progress: None,
        created_at: Utc::now().naive_utc(),
        started_at: None,
        completed_at: None,
    }
}

#[tokio::test]
async fn persisted_tasks_acquire_in_priority_order() {
    let queue = Arc::new(MemoryTaskQueue::with_limits(1, 1, 1, 1, 1));
    queue
        .submit_persisted(pending_task("low", TaskPriority::Low))
        .unwrap();
    queue
        .submit_persisted(pending_task("high", TaskPriority::High))
        .unwrap();

    assert!(
        tokio::time::timeout(
            Duration::from_millis(150),
            queue.acquire_task_with_cancel("low", CancellationToken::new()),
        )
        .await
        .is_err()
    );

    let high = queue
        .acquire_task_with_cancel("high", CancellationToken::new())
        .await
        .unwrap()
        .unwrap();
    high.complete();

    let low = queue
        .acquire_task_with_cancel("low", CancellationToken::new())
        .await
        .unwrap()
        .unwrap();
    low.complete();
}
