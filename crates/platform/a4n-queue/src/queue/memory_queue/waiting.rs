use std::collections::BinaryHeap;
use std::sync::Arc;

use dashmap::DashMap;
use parking_lot::RwLock;

use super::kind::{QueueKind, queue_kind_for_task_type};
use crate::queue::priority::{TaskInfo, TaskPriority, TaskStatus};

#[derive(Debug, Clone)]
pub(super) enum SpecificPop {
    Ready,
    Blocked,
    Missing,
}

#[derive(Debug, Default, Clone, Copy)]
pub(super) struct WaitingCounts {
    pub(super) llm_waiting: usize,
    pub(super) image_waiting: usize,
    pub(super) video_waiting: usize,
    pub(super) embedding_waiting: usize,
    pub(super) background_waiting: usize,
    pub(super) high_priority_waiting: usize,
    pub(super) medium_priority_waiting: usize,
    pub(super) low_priority_waiting: usize,
}

/// 等待中的任务项（用于优先队列排序）
#[derive(Debug, Clone, Eq, PartialEq)]
pub(super) struct WaitingTask {
    priority: TaskPriority,
    task_id: String,
    enqueued_at: std::time::Instant,
}

impl WaitingTask {
    pub(super) fn new(priority: TaskPriority, task_id: String) -> Self {
        Self {
            priority,
            task_id,
            enqueued_at: std::time::Instant::now(),
        }
    }
}

impl Ord for WaitingTask {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // 高优先级优先（反转派生的 Ord：High=0 < Low=2，但需要 High 先出）
        // 同优先级 FIFO（最早入队优先）
        other
            .priority
            .cmp(&self.priority)
            .then(other.enqueued_at.cmp(&self.enqueued_at))
    }
}

impl PartialOrd for WaitingTask {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

pub(super) fn pop_specific_if_next(
    waiting: &Arc<RwLock<BinaryHeap<WaitingTask>>>,
    tasks: &Arc<DashMap<String, TaskInfo>>,
    task_id: &str,
    kind: QueueKind,
) -> SpecificPop {
    let mut waiting = waiting.write();
    let mut remaining = Vec::new();
    let mut result = SpecificPop::Missing;

    while let Some(waiting_task) = waiting.pop() {
        let Some(task_info) = tasks.get(&waiting_task.task_id) else {
            continue;
        };
        if !matches!(task_info.status, TaskStatus::Pending) {
            continue;
        }
        if queue_kind_for_task_type(&task_info.task_type) != kind {
            remaining.push(waiting_task);
            continue;
        }

        if waiting_task.task_id == task_id {
            result = SpecificPop::Ready;
        } else {
            remaining.push(waiting_task);
            result = SpecificPop::Blocked;
        }
        break;
    }

    for waiting_task in remaining {
        waiting.push(waiting_task);
    }

    result
}

pub(super) fn remove_task(waiting: &Arc<RwLock<BinaryHeap<WaitingTask>>>, task_id: &str) -> bool {
    let mut waiting = waiting.write();
    let mut remaining = Vec::new();
    let mut removed = false;
    while let Some(waiting_task) = waiting.pop() {
        if waiting_task.task_id == task_id {
            removed = true;
        } else {
            remaining.push(waiting_task);
        }
    }
    for waiting_task in remaining {
        waiting.push(waiting_task);
    }
    removed
}

pub(super) fn counts(
    waiting: &Arc<RwLock<BinaryHeap<WaitingTask>>>,
    tasks: &Arc<DashMap<String, TaskInfo>>,
) -> WaitingCounts {
    let waiting = waiting.read();
    let mut counts = WaitingCounts::default();

    for waiting_task in waiting.iter() {
        let Some(task_info) = tasks.get(&waiting_task.task_id) else {
            continue;
        };
        if !matches!(task_info.status, TaskStatus::Pending) {
            continue;
        }

        match task_info.priority {
            TaskPriority::High => counts.high_priority_waiting += 1,
            TaskPriority::Medium => counts.medium_priority_waiting += 1,
            TaskPriority::Low => counts.low_priority_waiting += 1,
        }
        match queue_kind_for_task_type(&task_info.task_type) {
            QueueKind::Llm => counts.llm_waiting += 1,
            QueueKind::Image => counts.image_waiting += 1,
            QueueKind::Video => counts.video_waiting += 1,
            QueueKind::Embedding => counts.embedding_waiting += 1,
            QueueKind::Background => counts.background_waiting += 1,
        }
    }

    counts
}
