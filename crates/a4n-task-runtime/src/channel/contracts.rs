use a4n_domain::progress::ProgressMeta;
use serde::Serialize;

pub(super) const MAX_TASK_PROGRESS_EVENTS: usize = 200;

/// 任务进度
#[derive(Debug, Clone, Serialize)]
pub struct TaskProgress {
    pub current: u32,
    pub total: u32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<ProgressMeta>,
    /// 模型输出的活体信号：仅保留尾部片段，随每个 delta 覆盖写。
    ///
    /// 只存在于当前快照，**不进 `TaskProgressEvent` 历史缓冲**——delta 频率是每 token
    /// 一次，推成事件会冲爆环形缓冲并挤掉字段级里程碑。前端按固定间隔轮询快照，
    /// 天然完成降采样。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream_text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
}

impl TaskProgress {
    pub fn with_meta(
        current: u32,
        total: u32,
        message: impl Into<String>,
        meta: Option<ProgressMeta>,
    ) -> Self {
        Self {
            current,
            total,
            message: message.into(),
            meta,
            stream_text: None,
            updated_at: Some(chrono::Utc::now().to_rfc3339()),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct TaskProgressEvent {
    pub id: String,
    pub current: u32,
    pub total: u32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<ProgressMeta>,
    pub timestamp: String,
}

/// 任务快照
#[derive(Debug, Clone, Serialize)]
pub struct TaskSnapshot {
    pub task_id: String,
    pub project_id: Option<i64>,
    pub root_project_id: Option<i64>,
    pub source_snapshot_id: Option<i64>,
    pub task_scope: Option<String>,
    pub progress: TaskProgress,
    pub progress_events: Vec<TaskProgressEvent>,
}
