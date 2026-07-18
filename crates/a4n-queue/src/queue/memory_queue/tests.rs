use super::*;
use crate::queue::priority::{TaskPriority, TaskQueue, TaskStatus, TaskType};

#[tokio::test]
async fn test_submit_and_status() {
    let queue = MemoryTaskQueue::new(4, 2);
    let task_id = queue
        .submit(TaskType::LlmCall, TaskPriority::High, Some(1))
        .await
        .unwrap();

    let info = queue.status(&task_id).await.unwrap();
    assert_eq!(info.priority, TaskPriority::High);
    assert!(matches!(info.status, TaskStatus::Pending));
}

#[tokio::test]
async fn test_acquire_and_complete() {
    let queue = MemoryTaskQueue::new(4, 2);
    let task_id = queue
        .submit(TaskType::LlmCall, TaskPriority::High, None)
        .await
        .unwrap();

    let permit = queue.acquire(TaskType::LlmCall).await.unwrap();
    assert_eq!(permit.task_id(), &task_id);

    permit.complete();

    let info = queue.status(&task_id).await.unwrap();
    assert!(matches!(info.status, TaskStatus::Completed));
}

#[tokio::test]
async fn test_cancel() {
    let queue = MemoryTaskQueue::new(4, 2);
    let task_id = queue
        .submit(TaskType::LlmCall, TaskPriority::Low, None)
        .await
        .unwrap();

    queue.cancel(&task_id).await.unwrap();

    let info = queue.status(&task_id).await.unwrap();
    assert!(matches!(info.status, TaskStatus::Cancelled));
}

#[tokio::test]
async fn cancel_completed_task_preserves_terminal_status() {
    let queue = MemoryTaskQueue::new(4, 2);
    let task_id = queue
        .submit(TaskType::LlmCall, TaskPriority::High, None)
        .await
        .unwrap();

    let permit = queue.acquire(TaskType::LlmCall).await.unwrap();
    assert_eq!(permit.task_id(), &task_id);
    permit.complete();

    queue.cancel(&task_id).await.unwrap();

    let info = queue.status(&task_id).await.unwrap();
    assert!(matches!(info.status, TaskStatus::Completed));
}

#[tokio::test]
async fn completing_cancelled_running_task_preserves_cancelled_status() {
    let queue = MemoryTaskQueue::new(1, 1);
    let task_id = queue
        .submit(TaskType::LlmCall, TaskPriority::High, None)
        .await
        .unwrap();
    let permit = queue.acquire(TaskType::LlmCall).await.unwrap();
    assert_eq!(permit.task_id(), &task_id);

    queue.cancel(&task_id).await.unwrap();
    permit.complete();

    let info = queue.status(&task_id).await.unwrap();
    assert!(matches!(info.status, TaskStatus::Cancelled));

    let next_id = queue
        .submit(TaskType::LlmCall, TaskPriority::High, None)
        .await
        .unwrap();
    let next_permit = tokio::time::timeout(
        std::time::Duration::from_millis(250),
        queue.acquire(TaskType::LlmCall),
    )
    .await
    .expect("cancelled task permit should still release its slot")
    .unwrap();
    assert_eq!(next_permit.task_id(), &next_id);
    next_permit.complete();
}

#[tokio::test]
async fn cancelled_pending_task_is_not_acquired() {
    let queue = MemoryTaskQueue::new(4, 2);
    let cancelled_id = queue
        .submit(TaskType::LlmCall, TaskPriority::High, None)
        .await
        .unwrap();
    let next_id = queue
        .submit(TaskType::LlmCall, TaskPriority::Medium, None)
        .await
        .unwrap();

    queue.cancel(&cancelled_id).await.unwrap();

    let permit = queue.acquire(TaskType::LlmCall).await.unwrap();
    assert_eq!(permit.task_id(), next_id);
    permit.complete();
}

#[tokio::test]
async fn acquire_task_waits_for_its_priority_turn() {
    let queue = std::sync::Arc::new(MemoryTaskQueue::new(1, 1));
    let low_id = queue
        .submit(TaskType::LlmCall, TaskPriority::Low, None)
        .await
        .unwrap();
    let high_id = queue
        .submit(TaskType::LlmCall, TaskPriority::High, None)
        .await
        .unwrap();

    let low_queue = queue.clone();
    let low_id_for_task = low_id.clone();
    let low_wait = tokio::spawn(async move {
        tokio::time::timeout(
            std::time::Duration::from_millis(150),
            low_queue.acquire_task_with_cancel(
                &low_id_for_task,
                tokio_util::sync::CancellationToken::new(),
            ),
        )
        .await
    });

    assert!(low_wait.await.unwrap().is_err());

    let high = queue
        .acquire_task_with_cancel(&high_id, tokio_util::sync::CancellationToken::new())
        .await
        .unwrap()
        .expect("high priority task should acquire first");
    assert_eq!(high.task_id(), high_id);
    high.complete();

    let low = queue
        .acquire_task_with_cancel(&low_id, tokio_util::sync::CancellationToken::new())
        .await
        .unwrap()
        .expect("low priority task should acquire after high priority completes");
    assert_eq!(low.task_id(), low_id);
    low.complete();
}

#[tokio::test]
async fn test_queue_stats() {
    let queue = MemoryTaskQueue::new(4, 2);
    queue
        .submit(TaskType::LlmCall, TaskPriority::High, None)
        .await
        .unwrap();
    queue
        .submit(TaskType::LlmCall, TaskPriority::Medium, None)
        .await
        .unwrap();

    let stats = queue.queue_stats();
    assert_eq!(stats.high_priority_waiting, 1);
    assert_eq!(stats.medium_priority_waiting, 1);
}

#[tokio::test]
async fn test_priority_ordering() {
    let queue = MemoryTaskQueue::new(4, 2);

    queue
        .submit(TaskType::LlmCall, TaskPriority::Low, None)
        .await
        .unwrap();
    let high_id = queue
        .submit(TaskType::LlmCall, TaskPriority::High, None)
        .await
        .unwrap();

    let permit = queue.acquire(TaskType::LlmCall).await.unwrap();
    assert_eq!(permit.task_id(), &high_id);
    permit.complete();
}
