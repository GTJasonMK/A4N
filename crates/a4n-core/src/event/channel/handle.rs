use super::*;
use crate::event::progress::ProgressMeta;
use parking_lot::RwLock;
use std::collections::VecDeque;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

/// 活体信号只保留尾部片段：它用于证明「模型还在输出」，不用于阅读内容。
/// 按字符边界截断，避免切断多字节 UTF-8 序列。
const STREAM_TEXT_TAIL_CHARS: usize = 512;

fn tail_stream_text(text: &str) -> String {
    let start = text
        .char_indices()
        .rev()
        .nth(STREAM_TEXT_TAIL_CHARS - 1)
        .map_or(0, |(index, _)| index);
    text[start..].to_owned()
}

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
        let mut p = self.progress.write();
        let mut events = self.progress_events.write();
        if self.is_cancelled() {
            return;
        }
        let progress = TaskProgress {
            current,
            total,
            message: message.to_string(),
            meta: meta.clone(),
            // 活体信号跨里程碑保留：阶段推进不应让流式尾巴闪断。
            stream_text: p.stream_text.clone(),
            updated_at: Some(updated_at.clone()),
        };
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

    /// 写入模型输出的活体信号（流式尾巴），只更新当前快照。
    ///
    /// 刻意**不**推送 `TaskProgressEvent`：delta 频率是每 token 一次，
    /// 推成事件会冲爆环形缓冲并把字段级里程碑挤掉。前端按固定间隔轮询快照，
    /// 只会看到最近一次尾巴，正是活体信号所需。
    pub fn update_stream_text(&self, text: &str) {
        let mut p = self.progress.write();
        if self.is_cancelled() {
            return;
        }
        p.stream_text = Some(tail_stream_text(text));
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
