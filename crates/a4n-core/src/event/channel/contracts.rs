use crate::event::sse::{ProgressMeta, ProgressPhaseKind, ProgressTaskKind};
use serde::Serialize;

pub(super) const MAX_TASK_PROGRESS_EVENTS: usize = 200;

/// 任务类型
#[derive(Debug, Clone, Serialize)]
pub enum TaskType {
    /// LLM 调用（对话、生成等）
    LlmCall,
    /// 图片生成
    ImageGeneration,
    /// 视频生成
    VideoGeneration,
    /// Galgame 剧本管线
    GalgameGeneration,
    /// RAG 入库
    RagIngestion,
    /// 导出生成
    ExportGeneration,
    /// 嵌入模型下载
    EmbeddingDownload,
}

impl TaskType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::LlmCall => "llm_call",
            Self::ImageGeneration => "image_generation",
            Self::VideoGeneration => "video_generation",
            Self::GalgameGeneration => "galgame_generation",
            Self::RagIngestion => "rag_ingestion",
            Self::ExportGeneration => "export_generation",
            Self::EmbeddingDownload => "embedding_download",
        }
    }

    pub fn progress_meta(&self, message: &str) -> ProgressMeta {
        let (task_kind, stage_code, stage_label, phase_kind) = match self {
            Self::LlmCall => (
                ProgressTaskKind::LlmStructuredOutput,
                "llm_call",
                "模型生成",
                ProgressPhaseKind::ModelStream,
            ),
            Self::ImageGeneration => (
                ProgressTaskKind::ImageGeneration,
                "image_generation",
                "图片生成",
                ProgressPhaseKind::AssetGeneration,
            ),
            Self::VideoGeneration => (
                ProgressTaskKind::VideoGeneration,
                "video_generation",
                "视频生成",
                ProgressPhaseKind::AssetGeneration,
            ),
            Self::GalgameGeneration => (
                ProgressTaskKind::GalgameGeneration,
                "galgame_pipeline",
                "Galgame 管线",
                ProgressPhaseKind::ModelWait,
            ),
            Self::RagIngestion => (
                ProgressTaskKind::RagIngestion,
                "rag_ingestion",
                "RAG 入库",
                ProgressPhaseKind::Save,
            ),
            Self::ExportGeneration => (
                ProgressTaskKind::ExportGeneration,
                "export_generation",
                "导出生成",
                ProgressPhaseKind::Save,
            ),
            Self::EmbeddingDownload => (
                ProgressTaskKind::EmbeddingDownload,
                "embedding_download",
                "模型下载",
                ProgressPhaseKind::Transfer,
            ),
        };
        ProgressMeta::new(task_kind, stage_code, stage_label, phase_kind).with_detail(message)
    }
}

/// 任务进度
#[derive(Debug, Clone, Serialize)]
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
            updated_at: Some(chrono::Utc::now().to_rfc3339()),
        }
    }

    /// 进度百分比（0.0 ~ 1.0）
    pub fn percent(&self) -> f32 {
        if self.total == 0 {
            0.0
        } else {
            self.current as f32 / self.total as f32
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct TaskProgressEvent {
    pub id: String,
    pub task_id: String,
    pub current: u32,
    pub total: u32,
    pub message: String,
    pub percent: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<ProgressMeta>,
    pub timestamp: String,
}

/// 任务快照
#[derive(Debug, Clone, Serialize)]
pub struct TaskSnapshot {
    pub task_id: String,
    pub task_type: TaskType,
    pub project_id: Option<i64>,
    pub root_project_id: Option<i64>,
    pub source_snapshot_id: Option<i64>,
    pub task_scope: Option<String>,
    pub progress: TaskProgress,
    pub progress_events: Vec<TaskProgressEvent>,
    pub cancelled: bool,
}
