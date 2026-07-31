use super::*;
use crate::event::progress::ProgressMeta;
use parking_lot::RwLock;
use std::collections::VecDeque;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

/// 任务句柄
///
/// 持有取消令牌和进度信息，由 `TaskManager` 统一管理。
pub struct TaskHandle {
    pub task_id: String,
    pub owner_user_id: Option<i64>,
    pub cancel_token: CancellationToken,
    pub progress: Arc<RwLock<TaskProgress>>,
    pub progress_events: Arc<RwLock<VecDeque<TaskProgressEvent>>>,
    pub task_type: TaskKind,
    pub project_id: Option<i64>,
    pub root_project_id: Option<i64>,
    pub source_snapshot_id: Option<i64>,
    pub task_scope: Option<String>,
}

impl TaskHandle {
    /// 取消任务
    pub fn cancel(&self) {
        // 与进度写入使用同一锁顺序，保证 cancel 返回后不会再提交迟到进度。
        let _progress = self.progress.write();
        let _events = self.progress_events.write();
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
        )
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
        let mut events = self.progress_events.write();
        if self.is_cancelled() {
            return;
        }
        *p = progress.clone();

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
        events.push_back(event);
        while events.len() > MAX_TASK_PROGRESS_EVENTS {
            events.pop_front();
        }
    }

    pub fn update_progress_bytes(
        &self,
        current: u64,
        total: u64,
        progress_total: u32,
        message: &str,
    ) {
        let progress_total = progress_total.max(1);
        let scaled_current = if total > 0 {
            let scaled =
                (u128::from(current.min(total)) * u128::from(progress_total)) / u128::from(total);
            u32::try_from(scaled).expect("scaled byte progress is bounded by progress_total")
        } else {
            0
        };
        let meta =
            self.task_type
                .progress_meta(message)
                .with_unit("字节", Some(current), Some(total));
        self.update_progress_with_meta(scaled_current, progress_total, message, Some(meta));
    }
}
