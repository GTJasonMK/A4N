use std::sync::Arc;

use a4n_error::AppError;
use a4n_task_runtime::channel::TaskHandle;

pub(in crate::routes::commands) fn web_progress_reporter(
    task: Option<Arc<TaskHandle>>,
) -> impl Fn(u32, u32, &str) -> Result<(), AppError> + Send + Sync + 'static {
    move |current, total, message| {
        if let Some(handle) = task.as_ref() {
            if handle.is_cancelled() {
                return Err(AppError::Cancelled);
            }
            handle.update_progress(current, total, message);
        }
        Ok(())
    }
}

pub(in crate::routes::commands) fn web_fire_and_forget_progress_reporter(
    task: Option<Arc<TaskHandle>>,
) -> impl Fn(u32, u32, &str) + Send + Sync + 'static {
    move |current, total, message| {
        if let Some(handle) = task.as_ref() {
            handle.update_progress(current, total, message);
        }
    }
}

/// 活体信号出口，与桌面端 `WorkflowProgressTask::stream_text` 对称。
///
/// 只更新任务快照，不推进度事件——delta 频率是每 token 一次，
/// 推成事件会冲爆环形缓冲并挤掉字段级里程碑。
pub(in crate::routes::commands) fn web_stream_reporter(
    task: Option<Arc<TaskHandle>>,
) -> impl Fn(&str) + Send + Sync + 'static {
    move |text| {
        if let Some(handle) = task.as_ref() {
            handle.update_stream_text(text);
        }
    }
}
