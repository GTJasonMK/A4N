//! 内存任务队列：基于优先级的任务调度
//!
//! 使用 DashMap 管理任务状态，三个优先级通道（高/中/低）实现简单优先级调度。

use dashmap::DashMap;
use parking_lot::RwLock;
use std::collections::BinaryHeap;
use std::sync::Arc;
use tokio::sync::Semaphore;
use tokio_util::sync::CancellationToken;

mod acquire;
mod cancellation;
mod kind;
mod task_queue_impl;
mod waiting;

#[cfg(test)]
mod tests;

use super::priority::{TaskInfo, TaskProgress, TaskStatus};
use kind::QueueKind;
use waiting::WaitingTask;

/// 内存任务队列
///
/// 支持高/中/低三级优先级调度、并发信号量控制和取消令牌。
pub struct MemoryTaskQueue {
    /// 所有任务信息
    tasks: Arc<DashMap<String, TaskInfo>>,
    /// 优先级等待队列
    waiting: Arc<RwLock<BinaryHeap<WaitingTask>>>,
    /// 各类型并发槽位
    llm_semaphore: Arc<Semaphore>,
    image_semaphore: Arc<Semaphore>,
    video_semaphore: Arc<Semaphore>,
    embedding_semaphore: Arc<Semaphore>,
    background_semaphore: Arc<Semaphore>,
    max_llm_concurrent: usize,
    max_image_concurrent: usize,
    max_video_concurrent: usize,
    max_embedding_concurrent: usize,
    max_background_concurrent: usize,
    /// 取消令牌（task_id → token）
    cancel_tokens: Arc<DashMap<String, CancellationToken>>,
}

impl MemoryTaskQueue {
    /// 创建内存任务队列
    pub fn new(max_llm_concurrent: usize, max_image_concurrent: usize) -> Self {
        Self::with_limits(max_llm_concurrent, max_image_concurrent, 1, 2, 2)
    }

    /// 创建带完整槽位配置的内存任务队列
    pub fn with_limits(
        max_llm_concurrent: usize,
        max_image_concurrent: usize,
        max_video_concurrent: usize,
        max_embedding_concurrent: usize,
        max_background_concurrent: usize,
    ) -> Self {
        Self {
            tasks: Arc::new(DashMap::new()),
            waiting: Arc::new(RwLock::new(BinaryHeap::new())),
            llm_semaphore: Arc::new(Semaphore::new(max_llm_concurrent)),
            image_semaphore: Arc::new(Semaphore::new(max_image_concurrent)),
            video_semaphore: Arc::new(Semaphore::new(max_video_concurrent)),
            embedding_semaphore: Arc::new(Semaphore::new(max_embedding_concurrent)),
            background_semaphore: Arc::new(Semaphore::new(max_background_concurrent)),
            max_llm_concurrent,
            max_image_concurrent,
            max_video_concurrent,
            max_embedding_concurrent,
            max_background_concurrent,
            cancel_tokens: Arc::new(DashMap::new()),
        }
    }

    fn semaphore_for_kind(&self, kind: QueueKind) -> &Arc<Semaphore> {
        match kind {
            QueueKind::Llm => &self.llm_semaphore,
            QueueKind::Image => &self.image_semaphore,
            QueueKind::Video => &self.video_semaphore,
            QueueKind::Embedding => &self.embedding_semaphore,
            QueueKind::Background => &self.background_semaphore,
        }
    }

    /// 更新任务状态
    fn update_status(&self, task_id: &str, status: TaskStatus, progress: Option<TaskProgress>) {
        if let Some(mut task) = self.tasks.get_mut(task_id) {
            let now = chrono::Utc::now().naive_utc();
            match &status {
                TaskStatus::Running if task.started_at.is_none() => {
                    task.started_at = Some(now);
                }
                TaskStatus::Completed | TaskStatus::Failed { .. } | TaskStatus::Cancelled => {
                    task.completed_at = Some(now);
                }
                _ => {}
            }
            task.status = status;
            task.progress = progress;
        }
    }
}

impl Default for MemoryTaskQueue {
    fn default() -> Self {
        Self::new(4, 2)
    }
}

fn is_terminal_status(status: &TaskStatus) -> bool {
    matches!(
        status,
        TaskStatus::Completed
            | TaskStatus::Failed { .. }
            | TaskStatus::Cancelled
            | TaskStatus::Interrupted { .. }
    )
}
