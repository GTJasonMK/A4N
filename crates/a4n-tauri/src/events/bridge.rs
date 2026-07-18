//! 事件桥接：workflow:{task_id} 通道
//!
//! 将服务层产生的 SseEvent 推送到前端对应通道。
//! 基于 Tauri 的 emit 机制，实现多任务并发隔离。

use a4n_core::error::AppError;
use a4n_core::event::sse::{ProgressMeta, SseEvent};
use tauri::Emitter;
use tauri::Window;

/// 事件桥接器
///
/// 持有 Tauri Window 引用，将服务层事件转发到前端。
/// 使用 `workflow:{task_id}` 命名空间通道，确保多任务并发不冲突。
pub struct EventBridge {
    window: Window,
}

impl EventBridge {
    /// 创建事件桥接器
    pub fn new(window: Window) -> Self {
        Self { window }
    }

    /// 向 workflow:{task_id} 通道发送事件
    ///
    /// 前端通过 `listen("workflow:{task_id}", callback)` 监听。
    fn emit_workflow_event(&self, task_id: &str, event: SseEvent) -> Result<(), AppError> {
        let channel = SseEvent::workflow_channel(task_id);
        let result = serde_json::to_value(&event)
            .map_err(AppError::Serialization)
            .and_then(|payload| {
                self.window
                    .emit(&channel, payload)
                    .map_err(|error| AppError::Internal(error.into()))
            });
        if let Err(error) = &result {
            tracing::warn!(task_id, channel, %error, "Tauri workflow 事件发送失败");
        }
        result
    }

    pub fn emit_progress_with_meta(
        &self,
        task_id: &str,
        current: u32,
        total: u32,
        message: &str,
        meta: Option<ProgressMeta>,
    ) -> Result<(), AppError> {
        let event = match meta {
            Some(meta) => SseEvent::progress_with_meta(current, total, message, meta),
            None => SseEvent::progress(current, total, message),
        };
        self.emit_workflow_event(task_id, event)
    }

    /// 发送字节级进度事件。
    pub fn emit_progress_bytes(
        &self,
        task_id: &str,
        current: u64,
        total: u64,
        message: &str,
    ) -> Result<(), AppError> {
        self.emit_workflow_event(task_id, SseEvent::progress_bytes(current, total, message))
    }

    /// 发送完成事件
    pub fn emit_complete(&self, task_id: &str, summary: Option<String>) -> Result<(), AppError> {
        self.emit_workflow_event(task_id, SseEvent::complete(summary))
    }

    /// 发送错误事件
    pub fn emit_error(
        &self,
        task_id: &str,
        code: &str,
        message: &str,
        recoverable: bool,
    ) -> Result<(), AppError> {
        self.emit_workflow_event(task_id, SseEvent::error(code, message, recoverable))
    }

    /// 发送流式内容块
    pub fn emit_chunk(&self, task_id: &str, content: &str, index: u32) -> Result<(), AppError> {
        self.emit_workflow_event(task_id, SseEvent::chunk(content, index))
    }
}

impl std::fmt::Debug for EventBridge {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EventBridge").finish()
    }
}
