use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;
use tokio_util::sync::CancellationToken;

use super::{MAX_TERMINAL_TASKS, MemoryTaskQueue};
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

#[tokio::test]
async fn terminal_retention_is_bounded_without_evicting_active_tasks() {
    let queue = MemoryTaskQueue::with_limits(2, 1, 1, 1, 1);
    queue
        .submit_persisted(pending_task("running", TaskPriority::High))
        .unwrap();
    let running = queue
        .acquire_task_with_cancel("running", CancellationToken::new())
        .await
        .unwrap()
        .unwrap();
    queue
        .submit_persisted(pending_task("pending", TaskPriority::Low))
        .unwrap();

    for index in 0..MAX_TERMINAL_TASKS + 20 {
        let id = format!("completed-{index}");
        queue
            .submit_persisted(pending_task(&id, TaskPriority::High))
            .unwrap();
        let permit = queue
            .acquire_task_with_cancel(&id, CancellationToken::new())
            .await
            .unwrap()
            .unwrap();
        permit.complete();
    }
    assert_eq!(queue.tasks.len(), MAX_TERMINAL_TASKS + 2);
    assert!(!queue.tasks.contains_key("completed-0"));
    assert!(queue.tasks.contains_key("running"));
    assert!(queue.tasks.contains_key("pending"));
    assert_eq!(queue.queue_stats().llm_queue.running, 1);

    for index in 0..MAX_TERMINAL_TASKS + 20 {
        let id = format!("cancelled-{index}");
        queue
            .submit_persisted(pending_task(&id, TaskPriority::High))
            .unwrap();
        queue.cancel_task_now(&id).unwrap();
        queue
            .project_task_terminal(&id, TaskStatus::Cancelled)
            .unwrap();
    }
    assert_eq!(queue.terminal_tasks.lock().len(), MAX_TERMINAL_TASKS);
    assert_eq!(queue.tasks.len(), MAX_TERMINAL_TASKS + 2);
    assert!(queue.cancel_tokens.len() <= MAX_TERMINAL_TASKS + 2);
    assert_eq!(queue.queue_stats().llm_queue.waiting, 1);
    running.complete();
    queue.cancel_task_now("pending").unwrap();
    assert_eq!(queue.tasks.len(), MAX_TERMINAL_TASKS);
    assert_eq!(queue.queue_stats().llm_queue.running, 0);
}
