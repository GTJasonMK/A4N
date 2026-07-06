//! SSE 事件统一规范
//!
//! 所有流式接口使用统一的事件协议，通过 `workflow:{task_id}` 通道推送。

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProgressTaskKind {
    TextGeneration,
    LlmStructuredOutput,
    ImageGeneration,
    VideoGeneration,
    MangaGeneration,
    GalgameGeneration,
    RagIngestion,
    ImportAnalysis,
    ExportGeneration,
    EmbeddingDownload,
    BackgroundTask,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProgressPhaseKind {
    Prepare,
    Context,
    ModelConnect,
    ModelWait,
    ModelStream,
    StructuredOutput,
    Parse,
    Validate,
    Save,
    AssetGeneration,
    Transfer,
    Queue,
    Cleanup,
    Complete,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProgressSeverity {
    Info,
    Success,
    Warning,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProgressMeta {
    pub task_kind: ProgressTaskKind,
    pub stage_code: String,
    pub stage_label: String,
    pub phase_kind: ProgressPhaseKind,
    pub detail: Option<String>,
    pub unit_label: Option<String>,
    pub item_index: Option<u64>,
    pub item_total: Option<u64>,
    pub parent_stage_code: Option<String>,
    pub severity: ProgressSeverity,
    pub estimated: bool,
    pub persistent: bool,
}

impl ProgressMeta {
    pub fn new(
        task_kind: ProgressTaskKind,
        stage_code: impl Into<String>,
        stage_label: impl Into<String>,
        phase_kind: ProgressPhaseKind,
    ) -> Self {
        Self {
            task_kind,
            stage_code: stage_code.into(),
            stage_label: stage_label.into(),
            phase_kind,
            detail: None,
            unit_label: None,
            item_index: None,
            item_total: None,
            parent_stage_code: None,
            severity: ProgressSeverity::Info,
            estimated: false,
            persistent: false,
        }
    }

    pub fn with_detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }

    pub fn with_unit(
        mut self,
        unit_label: impl Into<String>,
        item_index: Option<u64>,
        item_total: Option<u64>,
    ) -> Self {
        self.unit_label = Some(unit_label.into());
        self.item_index = item_index;
        self.item_total = item_total;
        self
    }
}

/// 统一 SSE 事件枚举
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum SseEvent {
    /// 进度更新
    Progress {
        current: u64,
        total: u64,
        message: String,
        percent: f32,
        #[serde(skip_serializing_if = "Option::is_none")]
        meta: Option<ProgressMeta>,
    },
    /// 内容流式块
    Chunk { content: String, index: u32 },
    /// 完成
    Complete { summary: Option<String> },
    /// 错误
    Error {
        code: String,
        message: String,
        recoverable: bool,
    },
}

impl SseEvent {
    pub fn workflow_channel(task_id: &str) -> String {
        format!("workflow:{task_id}")
    }

    pub fn progress(current: u32, total: u32, message: &str) -> Self {
        Self::progress_bytes_with_meta(u64::from(current), u64::from(total), message, None)
    }

    pub fn progress_bytes(current: u64, total: u64, message: &str) -> Self {
        Self::progress_bytes_with_meta(
            current,
            total,
            message,
            Some(
                ProgressMeta::new(
                    ProgressTaskKind::BackgroundTask,
                    "transfer",
                    "传输进度",
                    ProgressPhaseKind::Transfer,
                )
                .with_detail(message)
                .with_unit("字节", Some(current), Some(total)),
            ),
        )
    }

    pub fn progress_with_meta(current: u32, total: u32, message: &str, meta: ProgressMeta) -> Self {
        Self::progress_bytes_with_meta(u64::from(current), u64::from(total), message, Some(meta))
    }

    pub fn progress_bytes_with_meta(
        current: u64,
        total: u64,
        message: &str,
        meta: Option<ProgressMeta>,
    ) -> Self {
        Self::Progress {
            current,
            total,
            message: message.to_string(),
            percent: if total > 0 {
                current as f32 / total as f32
            } else {
                0.0
            },
            meta,
        }
    }

    pub fn chunk(content: &str, index: u32) -> Self {
        Self::Chunk {
            content: content.to_string(),
            index,
        }
    }

    pub fn complete(summary: Option<String>) -> Self {
        Self::Complete { summary }
    }

    pub fn error(code: &str, message: &str, recoverable: bool) -> Self {
        Self::Error {
            code: code.to_string(),
            message: message.to_string(),
            recoverable,
        }
    }
}
