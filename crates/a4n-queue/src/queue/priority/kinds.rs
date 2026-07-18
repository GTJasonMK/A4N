use a4n_core::error::AppError;
use serde::{Deserialize, Serialize};

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
