use super::*;
use crate::event::sse::ProgressMeta;
use parking_lot::RwLock;
use std::collections::VecDeque;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

/// 任务句柄
///
/// 持有取消令牌和进度信息，由 `TaskManager` 统一管理。
pub struct TaskHandle {
    pub task_id: String,
    pub cancel_token: CancellationToken,
    pub progress: Arc<RwLock<TaskProgress>>,
    pub progress_events: Arc<RwLock<VecDeque<TaskProgressEvent>>>,
    pub task_type: TaskType,
    pub project_id: Option<i64>,
    pub root_project_id: Option<i64>,
    pub source_snapshot_id: Option<i64>,
    pub task_scope: Option<String>,
}

impl TaskHandle {
    /// 取消任务
    pub fn cancel(&self) {
        self.cancel_token.cancel();
    }

    /// 检查是否已被取消
    pub fn is_cancelled(&self) -> bool {
        self.cancel_token.is_cancelled()
    }

    /// 更新进度
    pub fn update_progress(&self, current: u32, total: u32, message: &str) {
        self.update_progress_with_meta(
            current,
            total,
            message,
            Some(self.task_type.progress_meta(message)),
        );
    }

    pub fn update_progress_with_meta(
        &self,
        current: u32,
        total: u32,
        message: &str,
        meta: Option<ProgressMeta>,
    ) {
        let updated_at = chrono::Utc::now().to_rfc3339();
        let progress = TaskProgress {
            current,
            total,
            message: message.to_string(),
            meta: meta.clone(),
            updated_at: Some(updated_at.clone()),
        };
        let mut p = self.progress.write();
        *p = progress.clone();
        drop(p);

        let event = TaskProgressEvent {
            id: format!("{}-{}", self.task_id, updated_at),
            task_id: self.task_id.clone(),
            current,
            total,
            message: message.to_string(),
            percent: progress.percent() * 100.0,
            meta,
            timestamp: updated_at,
        };
        let mut events = self.progress_events.write();
        events.push_back(event);
        while events.len() > MAX_TASK_PROGRESS_EVENTS {
            events.pop_front();
        }
    }
}
