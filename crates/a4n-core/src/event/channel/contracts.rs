use crate::{
    error::AppError,
    event::progress::{ProgressMeta, ProgressPhaseKind},
};
use serde::Serialize;

pub(super) const MAX_TASK_PROGRESS_EVENTS: usize = 200;

/// 进程内任务与持久队列共用的 canonical 任务种类。
///
/// 该合同位于 core，避免 queue、runtime、Server 和 Tauri 各自维护一套任务枚举。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, serde::Deserialize)]
pub enum TaskKind {
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
    /// 视频轮询
    VideoPoll,
    /// 来源同步
    SourceSync,
    /// 成片组装
    Assembly,
    /// 嵌入调用
    EmbeddingCall,
    /// RAG 入库
    ImportAnalysis,
    /// 漫画生成
    MangaGeneration,
    /// 文本生成
    TextGeneration,
}

impl TaskKind {
    const ALL: [Self; 14] = [
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
        Self::GalgameGeneration,
        Self::MangaGeneration,
        Self::TextGeneration,
    ];

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::LlmCall => "llm_call",
            Self::ImageGeneration => "image_generation",
            Self::VideoGeneration => "video_generation",
            Self::GalgameGeneration => "galgame_generation",
            Self::RagIngestion => "rag_ingestion",
            Self::ExportGeneration => "export_generation",
            Self::EmbeddingDownload => "embedding_download",
            Self::VideoPoll => "video_poll",
            Self::SourceSync => "source_sync",
            Self::Assembly => "assembly",
            Self::EmbeddingCall => "embedding_call",
            Self::ImportAnalysis => "import_analysis",
            Self::MangaGeneration => "manga_generation",
            Self::TextGeneration => "text_generation",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        Self::ALL
            .iter()
            .copied()
            .find(|task_kind| task_kind.as_str() == value)
    }

    pub fn parse_storage(value: &str) -> Result<Self, AppError> {
        Self::parse(value)
            .ok_or_else(|| AppError::InvalidArgument(format!("未知任务类型: {value}")))
    }

    pub fn progress_meta(&self, message: &str) -> ProgressMeta {
        let (task_kind, stage_code, stage_label, phase_kind) = match self {
            Self::LlmCall => (
                self.as_str(),
                "llm_call",
                "模型生成",
                ProgressPhaseKind::ModelStream,
            ),
            Self::ImageGeneration => (
                self.as_str(),
                "image_generation",
                "图片生成",
                ProgressPhaseKind::AssetGeneration,
            ),
            Self::VideoGeneration => (
                self.as_str(),
                "video_generation",
                "视频生成",
                ProgressPhaseKind::AssetGeneration,
            ),
            Self::GalgameGeneration => (
                self.as_str(),
                "galgame_pipeline",
                "Galgame 管线",
                ProgressPhaseKind::ModelWait,
            ),
            Self::RagIngestion => (
                self.as_str(),
                "rag_ingestion",
                "RAG 入库",
                ProgressPhaseKind::Save,
            ),
            Self::ExportGeneration => (
                self.as_str(),
                "export_generation",
                "导出生成",
                ProgressPhaseKind::Save,
            ),
            Self::EmbeddingDownload => (
                self.as_str(),
                "embedding_download",
                "模型下载",
                ProgressPhaseKind::Transfer,
            ),
            Self::VideoPoll => (
                self.as_str(),
                "video_poll",
                "视频轮询",
                ProgressPhaseKind::ModelWait,
            ),
            Self::SourceSync => (
                self.as_str(),
                "source_sync",
                "来源同步",
                ProgressPhaseKind::Save,
            ),
            Self::Assembly => (
                self.as_str(),
                "assembly",
                "成片组装",
                ProgressPhaseKind::AssetGeneration,
            ),
            Self::EmbeddingCall => (
                self.as_str(),
                "embedding_call",
                "嵌入调用",
                ProgressPhaseKind::ModelWait,
            ),
            Self::ImportAnalysis => (
                self.as_str(),
                "import_analysis",
                "导入分析",
                ProgressPhaseKind::Parse,
            ),
            Self::MangaGeneration => (
                self.as_str(),
                "manga_generation",
                "漫画生成",
                ProgressPhaseKind::AssetGeneration,
            ),
            Self::TextGeneration => (
                self.as_str(),
                "text_generation",
                "文本生成",
                ProgressPhaseKind::ModelStream,
            ),
        };
        ProgressMeta::new(task_kind, stage_code, stage_label, phase_kind).with_detail(message)
    }
}

impl std::str::FromStr for TaskKind {
    type Err = AppError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::parse_storage(value)
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
    pub task_type: TaskKind,
    pub project_id: Option<i64>,
    pub root_project_id: Option<i64>,
    pub source_snapshot_id: Option<i64>,
    pub task_scope: Option<String>,
    pub progress: TaskProgress,
    pub progress_events: Vec<TaskProgressEvent>,
    pub cancelled: bool,
}
