//! Pure task-queue contracts.

use crate::progress::{ProgressMeta, ProgressPhaseKind};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Resolved provider configuration identity; the key must not contain credentials.
#[derive(Debug, Clone)]
pub struct LlmConcurrencyLimit {
    pub key: String,
    pub max_concurrent: usize,
}

/// Task kinds shared by persistence, scheduling, and host progress reporting.
///
/// This is a domain contract: it deliberately does not depend on the in-process
/// task manager or on database error types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskKind {
    LlmCall,
    ImageGeneration,
    VideoGeneration,
    GalgameGeneration,
    RagIngestion,
    ExportGeneration,
    EmbeddingDownload,
    VideoPoll,
    SourceSync,
    Assembly,
    EmbeddingCall,
    ImportAnalysis,
    MangaGeneration,
    TextGeneration,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("参数无效：未知任务类型: {0}")]
pub struct TaskKindParseError(pub String);

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

    pub fn as_str(self) -> &'static str {
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

    pub fn parse_storage(value: &str) -> Result<Self, TaskKindParseError> {
        Self::parse(value).ok_or_else(|| TaskKindParseError(value.to_string()))
    }

    pub fn progress_meta(self, message: &str) -> ProgressMeta {
        let (stage_code, stage_label, phase_kind) = match self {
            Self::LlmCall => ("llm_call", "模型生成", ProgressPhaseKind::ModelStream),
            Self::ImageGeneration => (
                "image_generation",
                "图片生成",
                ProgressPhaseKind::AssetGeneration,
            ),
            Self::VideoGeneration => (
                "video_generation",
                "视频生成",
                ProgressPhaseKind::AssetGeneration,
            ),
            Self::GalgameGeneration => (
                "galgame_pipeline",
                "Galgame 管线",
                ProgressPhaseKind::ModelWait,
            ),
            Self::RagIngestion => ("rag_ingestion", "RAG 入库", ProgressPhaseKind::Save),
            Self::ExportGeneration => ("export_generation", "导出生成", ProgressPhaseKind::Save),
            Self::EmbeddingDownload => (
                "embedding_download",
                "模型下载",
                ProgressPhaseKind::Transfer,
            ),
            Self::VideoPoll => ("video_poll", "视频轮询", ProgressPhaseKind::ModelWait),
            Self::SourceSync => ("source_sync", "来源同步", ProgressPhaseKind::Save),
            Self::Assembly => ("assembly", "成片组装", ProgressPhaseKind::AssetGeneration),
            Self::EmbeddingCall => ("embedding_call", "嵌入调用", ProgressPhaseKind::ModelWait),
            Self::ImportAnalysis => ("import_analysis", "导入分析", ProgressPhaseKind::Parse),
            Self::MangaGeneration => (
                "manga_generation",
                "漫画生成",
                ProgressPhaseKind::AssetGeneration,
            ),
            Self::TextGeneration => (
                "text_generation",
                "文本生成",
                ProgressPhaseKind::ModelStream,
            ),
        };
        ProgressMeta::new(self.as_str(), stage_code, stage_label, phase_kind).with_detail(message)
    }
}

impl std::str::FromStr for TaskKind {
    type Err = TaskKindParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::parse_storage(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersistTaskQueueRecordInput {
    pub task_id: String,
    pub task_type: String,
    pub priority: String,
    pub project_id: Option<i64>,
    pub root_project_id: Option<i64>,
    pub source_snapshot_id: Option<i64>,
    pub task_scope: Option<String>,
    pub workflow_run_id: Option<i64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn task_kind_storage_round_trip_has_one_domain_owner() {
        for task_kind in TaskKind::ALL {
            assert_eq!(TaskKind::parse(task_kind.as_str()), Some(task_kind));
            assert_eq!(task_kind.as_str().parse::<TaskKind>().unwrap(), task_kind);
        }
    }

    #[test]
    fn task_kind_parse_error_keeps_storage_context() {
        let error = TaskKind::parse_storage("unknown_task_kind").unwrap_err();
        assert_eq!(
            error.to_string(),
            "参数无效：未知任务类型: unknown_task_kind"
        );
    }
}
