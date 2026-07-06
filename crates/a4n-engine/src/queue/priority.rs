//! TaskQueue trait：优先级任务调度
//!
//! 支持高/中/低优先级调度和并发控制（信号量）。

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio_util::sync::CancellationToken;

use a4n_core::error::AppError;
use a4n_core::event::sse::ProgressMeta;

/// 任务优先级
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum TaskPriority {
    /// 用户交互：灵感对话、章节生成
    High,
    /// 正常工作流：大纲生成、蓝图生成
    Medium,
    /// 后台任务：RAG 入库、导入分析
    Low,
}

impl TaskPriority {
    const ALL: [Self; 3] = [Self::High, Self::Medium, Self::Low];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::High => "high",
            Self::Medium => "medium",
            Self::Low => "low",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        Self::ALL
            .iter()
            .copied()
            .find(|priority| priority.as_str() == value)
    }

    pub fn parse_storage(value: &str) -> Result<Self, AppError> {
        Self::parse(value)
            .ok_or_else(|| AppError::InvalidArgument(format!("未知任务优先级: {value}")))
    }
}

/// 任务状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskStatus {
    Pending,
    Running,
    Completed,
    Failed { error: String },
    Cancelled,
    Interrupted { reason: Option<String> },
}

impl TaskStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Running => "running",
            Self::Completed => "completed",
            Self::Failed { .. } => "failed",
            Self::Cancelled => "cancelled",
            Self::Interrupted { .. } => "interrupted",
        }
    }

    pub fn parse_storage(value: &str, error_message: Option<&str>) -> Result<Self, AppError> {
        match value {
            "pending" => Ok(Self::Pending),
            "running" => Ok(Self::Running),
            "completed" => Ok(Self::Completed),
            "cancelled" => Ok(Self::Cancelled),
            "interrupted" => Ok(Self::Interrupted {
                reason: error_message.map(ToString::to_string),
            }),
            "failed" => {
                let error = error_message.ok_or_else(|| {
                    AppError::InvalidArgument("失败任务缺少 error_message".to_string())
                })?;
                Ok(Self::Failed {
                    error: error.to_string(),
                })
            }
            _ => Err(AppError::InvalidArgument(format!("未知任务状态: {value}"))),
        }
    }
}

/// 任务类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskType {
    LlmCall,
    ImageGeneration,
    VideoGeneration,
    VideoPoll,
    SourceSync,
    Assembly,
    EmbeddingCall,
    RagIngestion,
    ImportAnalysis,
    ExportGeneration,
    EmbeddingDownload,
}

impl TaskType {
    const ALL: [Self; 11] = [
        Self::LlmCall,
        Self::ImageGeneration,
        Self::VideoGeneration,
        Self::VideoPoll,
        Self::SourceSync,
        Self::Assembly,
        Self::EmbeddingCall,
        Self::RagIngestion,
        Self::ImportAnalysis,
        Self::ExportGeneration,
        Self::EmbeddingDownload,
    ];

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::LlmCall => "llm_call",
            Self::ImageGeneration => "image_generation",
            Self::VideoGeneration => "video_generation",
            Self::VideoPoll => "video_poll",
            Self::SourceSync => "source_sync",
            Self::Assembly => "assembly",
            Self::EmbeddingCall => "embedding_call",
            Self::RagIngestion => "rag_ingestion",
            Self::ImportAnalysis => "import_analysis",
            Self::ExportGeneration => "export_generation",
            Self::EmbeddingDownload => "embedding_download",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        Self::ALL
            .iter()
            .copied()
            .find(|task_type| task_type.as_str() == value)
    }

    pub fn parse_storage(value: &str) -> Result<Self, AppError> {
        Self::parse(value)
            .ok_or_else(|| AppError::InvalidArgument(format!("未知任务类型: {value}")))
    }
}

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
    pub task_type: TaskType,
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

/// TaskQueue trait
///
/// 统一的任务调度接口，支持单任务提交、取消和状态查询。
#[async_trait]
pub trait TaskQueue: Send + Sync {
    /// 提交单个任务，返回任务 ID
    async fn submit(
        &self,
        task_type: TaskType,
        priority: TaskPriority,
        project_id: Option<i64>,
    ) -> Result<String, AppError>;

    /// 获取执行许可（阻塞直到有可用槽位，高优先级优先）
    async fn acquire(&self, task_type: TaskType) -> Result<TaskPermit, AppError>;

    /// 获取执行许可（支持取消令牌）
    async fn acquire_with_cancel(
        &self,
        task_type: TaskType,
        cancel_token: CancellationToken,
    ) -> Result<Option<TaskPermit>, AppError>;

    /// 查询任务状态
    async fn status(&self, task_id: &str) -> Result<TaskInfo, AppError>;

    /// 取消任务（级联取消依赖它的后继任务）
    async fn cancel(&self, task_id: &str) -> Result<(), AppError>;

    /// 获取队列状态概览
    fn queue_stats(&self) -> QueueStats;

    /// 获取任务历史（按项目过滤）
    async fn history(
        &self,
        project_id: Option<i64>,
        offset: u64,
        limit: u64,
    ) -> Result<Vec<TaskInfo>, AppError>;
}

/// 任务许可（RAII 自动释放）
///
/// 获取许可后必须调用 `complete()` 或 `fail()` 释放槽位。
pub struct TaskPermit {
    task_id: String,
    task_type: TaskType,
    priority: TaskPriority,
    on_complete: Option<Box<dyn FnOnce(TaskStatus) + Send>>,
}

impl TaskPermit {
    pub fn new(
        task_id: String,
        task_type: TaskType,
        priority: TaskPriority,
        on_complete: Box<dyn FnOnce(TaskStatus) + Send>,
    ) -> Self {
        Self {
            task_id,
            task_type,
            priority,
            on_complete: Some(on_complete),
        }
    }

    pub fn task_id(&self) -> &str {
        &self.task_id
    }

    pub fn task_type(&self) -> &TaskType {
        &self.task_type
    }

    pub fn priority(&self) -> TaskPriority {
        self.priority
    }

    /// 标记任务完成（释放槽位）
    pub fn complete(mut self) {
        if let Some(on_complete) = self.on_complete.take() {
            on_complete(TaskStatus::Completed);
        }
    }

    /// 标记任务失败（释放槽位）
    pub fn fail(mut self, error: String) {
        if let Some(on_complete) = self.on_complete.take() {
            on_complete(TaskStatus::Failed { error });
        }
    }
}

impl Drop for TaskPermit {
    fn drop(&mut self) {
        if let Some(on_complete) = self.on_complete.take() {
            on_complete(TaskStatus::Failed {
                error: "任务许可被释放但未显式完成".to_string(),
            });
        }
    }
}

impl std::fmt::Debug for TaskPermit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TaskPermit")
            .field("task_id", &self.task_id)
            .field("task_type", &self.task_type)
            .field("priority", &self.priority)
            .finish()
    }
}
