use super::{TaskKind, TaskPriority, TaskStatus};
use a4n_core::event::progress::ProgressMeta;
use serde::{Deserialize, Serialize};

/// 任务进度
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskProgress {
    pub current: u32,
    pub total: u32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<ProgressMeta>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
}

impl TaskProgress {
    pub fn percent(&self) -> f32 {
        if self.total == 0 {
            0.0
        } else {
            self.current as f32 / self.total as f32
        }
    }
}

/// 任务信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskInfo {
    pub id: String,
    pub task_type: TaskKind,
    pub priority: TaskPriority,
    pub status: TaskStatus,
    pub project_id: Option<i64>,
    pub root_project_id: Option<i64>,
    pub source_snapshot_id: Option<i64>,
    pub task_scope: Option<String>,
    pub progress: Option<TaskProgress>,
    pub created_at: chrono::NaiveDateTime,
    pub started_at: Option<chrono::NaiveDateTime>,
    pub completed_at: Option<chrono::NaiveDateTime>,
}

/// 队列槽位统计
#[derive(Debug, Clone, Serialize)]
pub struct QueueSlotStats {
    pub max_concurrent: usize,
    pub running: usize,
    pub waiting: usize,
}

/// 队列统计
#[derive(Debug, Clone, Serialize)]
pub struct QueueStats {
    pub llm_queue: QueueSlotStats,
    pub image_queue: QueueSlotStats,
    pub video_queue: QueueSlotStats,
    pub embedding_queue: QueueSlotStats,
    pub background_queue: QueueSlotStats,
    pub high_priority_waiting: usize,
    pub medium_priority_waiting: usize,
    pub low_priority_waiting: usize,
}
