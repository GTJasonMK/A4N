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
async fn model_calls_use_their_own_limits_even_inside_a_workflow_slot() {
    let queue = MemoryTaskQueue::with_limits(1, 1, 1, 1, 1);
    let mut outer = pending_task("workflow", TaskPriority::Medium);
    outer.task_type = TaskKind::TextGeneration;
    queue.submit_persisted(outer).unwrap();
    let outer = queue
        .acquire_task_with_cancel("workflow", CancellationToken::new())
        .await
        .unwrap()
        .unwrap();
    for id in ["a-1", "a-2", "b-1"] {
        queue
            .submit_persisted(pending_task(id, TaskPriority::Medium))
            .unwrap();
    }
    let limit = |key: &str| a4n_domain::task_queue::LlmConcurrencyLimit {
        key: key.to_string(),
        max_concurrent: 1,
    };
    let first = tokio::time::timeout(
        Duration::from_millis(100),
        crate::with_llm_slot(queue.acquire_llm_call_with_cancel(
            "a-1",
            CancellationToken::new(),
            limit("a"),
        )),
    )
    .await
    .unwrap()
    .unwrap()
    .unwrap();
    assert!(
        tokio::time::timeout(
            Duration::from_millis(10),
            crate::with_llm_slot(queue.acquire_llm_call_with_cancel(
                "a-2",
                CancellationToken::new(),
                limit("a")
            ))
        )
        .await
        .is_err()
    );
    let other = tokio::time::timeout(
        Duration::from_millis(100),
        crate::with_llm_slot(queue.acquire_llm_call_with_cancel(
            "b-1",
            CancellationToken::new(),
            limit("b"),
        )),
    )
    .await
    .unwrap()
    .unwrap()
    .unwrap();
    queue.cancel_task_now("a-2").unwrap();
    first.complete();
    other.complete();
    outer.complete();
    assert_eq!(queue.queue_stats().llm_queue.running, 0);
}

#[tokio::test]
async fn standalone_model_calls_still_obey_process_admission() {
    let queue = MemoryTaskQueue::with_limits(1, 1, 1, 1, 1);
    for id in ["a", "b"] {
        queue
            .submit_persisted(pending_task(id, TaskPriority::Medium))
            .unwrap();
    }
    let limit = |key: &str| a4n_domain::task_queue::LlmConcurrencyLimit {
        key: key.into(),
        max_concurrent: 5,
    };
    let first = queue
        .acquire_llm_call_with_cancel("a", CancellationToken::new(), limit("a"))
        .await
        .unwrap()
        .unwrap();
    assert!(
        tokio::time::timeout(
            Duration::from_millis(10),
            queue.acquire_llm_call_with_cancel("b", CancellationToken::new(), limit("b"))
        )
        .await
        .is_err()
    );
    first.complete();
    let second = queue
        .acquire_llm_call_with_cancel("b", CancellationToken::new(), limit("b"))
        .await
        .unwrap()
        .unwrap();
    second.complete();
    assert_eq!(queue.queue_stats().llm_queue.running, 0);
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
