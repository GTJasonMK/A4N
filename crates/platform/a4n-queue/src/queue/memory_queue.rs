//! 内存任务队列：基于优先级的任务调度
//!
//! 使用 DashMap 管理任务状态，三个优先级通道（高/中/低）实现简单优先级调度。

use dashmap::DashMap;
use parking_lot::{Mutex, RwLock};
use std::collections::{BinaryHeap, VecDeque};
use std::sync::Arc;
use tokio::sync::Semaphore;
use tokio_util::sync::CancellationToken;

mod acquire;
mod cancellation;
mod kind;
mod llm_limits;
mod waiting;

#[cfg(test)]
mod tests;

use super::priority::{QueueSlotStats, QueueStats, TaskInfo, TaskProgress, TaskStatus};
use kind::QueueKind;
use waiting::WaitingTask;

const MAX_TERMINAL_TASKS: usize = 1024;

/// 内存任务队列
///
/// 支持高/中/低三级优先级调度、并发信号量控制和取消令牌。
#[derive(Clone)]
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
    llm_limits: Arc<llm_limits::LlmLimits>,
    max_image_concurrent: usize,
    max_video_concurrent: usize,
    max_embedding_concurrent: usize,
    max_background_concurrent: usize,
    /// 取消令牌（task_id → token）
    cancel_tokens: Arc<DashMap<String, CancellationToken>>,
    /// The database owns history; retain only recent terminal identities for idempotency.
    terminal_tasks: Arc<Mutex<VecDeque<String>>>,
}

impl MemoryTaskQueue {
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
            llm_limits: Arc::default(),
            max_image_concurrent,
            max_video_concurrent,
            max_embedding_concurrent,
            max_background_concurrent,
            cancel_tokens: Arc::new(DashMap::new()),
            terminal_tasks: Arc::new(Mutex::new(VecDeque::new())),
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

    /// 将数据库已经分配 id 的 pending task 登记到同一内存队列。
    pub fn submit_persisted(&self, task: TaskInfo) -> Result<(), a4n_error::AppError> {
        if !matches!(&task.status, TaskStatus::Pending) {
            return Err(a4n_error::AppError::InvalidState {
                current: task.status.as_str().to_string(),
                operation: "登记持久化任务到内存队列".to_string(),
            });
        }

        let mut waiting = self.waiting.write();
        if let Some(existing) = self.tasks.get(&task.id) {
            let same_identity = existing.task_type == task.task_type
                && existing.priority == task.priority
                && existing.project_id == task.project_id
                && existing.root_project_id == task.root_project_id
                && existing.source_snapshot_id == task.source_snapshot_id
                && existing.task_scope == task.task_scope;
            return if same_identity {
                Ok(())
            } else {
                Err(a4n_error::AppError::InvalidState {
                    current: format!("task {} 已以其他队列元数据登记", task.id),
                    operation: "重复登记持久化任务".to_string(),
                })
            };
        }

        let task_id = task.id.clone();
        let priority = task.priority;
        self.tasks.insert(task_id.clone(), task);
        self.cancel_tokens
            .insert(task_id.clone(), CancellationToken::new());
        waiting.push(WaitingTask::new(priority, task_id));
        Ok(())
    }

    pub fn queue_stats(&self) -> QueueStats {
        let mut counts = waiting::counts(&self.waiting, &self.tasks);
        let [high, medium, low] = self.llm_limits.waiting_by_priority();
        counts.llm_waiting += high + medium + low;
        counts.high_priority_waiting += high;
        counts.medium_priority_waiting += medium;
        counts.low_priority_waiting += low;
        QueueStats {
            llm_queue: QueueSlotStats {
                max_concurrent: self.max_llm_concurrent,
                running: self.max_llm_concurrent - self.llm_semaphore.available_permits(),
                waiting: counts.llm_waiting,
            },
            image_queue: QueueSlotStats {
                max_concurrent: self.max_image_concurrent,
                running: self.max_image_concurrent - self.image_semaphore.available_permits(),
                waiting: counts.image_waiting,
            },
            video_queue: QueueSlotStats {
                max_concurrent: self.max_video_concurrent,
                running: self.max_video_concurrent - self.video_semaphore.available_permits(),
                waiting: counts.video_waiting,
            },
            embedding_queue: QueueSlotStats {
                max_concurrent: self.max_embedding_concurrent,
                running: self.max_embedding_concurrent
                    - self.embedding_semaphore.available_permits(),
                waiting: counts.embedding_waiting,
            },
            background_queue: QueueSlotStats {
                max_concurrent: self.max_background_concurrent,
                running: self.max_background_concurrent
                    - self.background_semaphore.available_permits(),
                waiting: counts.background_waiting,
            },
            high_priority_waiting: counts.high_priority_waiting,
            medium_priority_waiting: counts.medium_priority_waiting,
            low_priority_waiting: counts.low_priority_waiting,
        }
    }

    /// 更新任务状态
    fn update_status(&self, task_id: &str, status: TaskStatus, progress: Option<TaskProgress>) {
        if let Some(mut task) = self.tasks.get_mut(task_id) {
            let became_terminal = is_terminal_status(&status) && !is_terminal_status(&task.status);
            let now = chrono::Utc::now().naive_utc();
            if matches!(status, TaskStatus::Running) && task.started_at.is_none() {
                task.started_at = Some(now);
            } else if is_terminal_status(&status) {
                task.completed_at = Some(now);
            }
            task.status = status;
            task.progress = progress;
            drop(task);
            if became_terminal {
                self.retain_terminal_task(task_id);
            }
        }
    }

    fn retain_terminal_task(&self, task_id: &str) {
        let mut terminal_tasks = self.terminal_tasks.lock();
        terminal_tasks.push_back(task_id.to_string());
        while terminal_tasks.len() > MAX_TERMINAL_TASKS {
            if let Some(expired_id) = terminal_tasks.pop_front() {
                self.tasks.remove(&expired_id);
                self.cancel_tokens.remove(&expired_id);
            }
        }
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
