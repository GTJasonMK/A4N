//! 任务通道管理
//!
//! 基于 task_id 命名空间的通道系统，支持任务注册、取消和进度跟踪。
//! 所有流式工作流通过 `workflow:{task_id}` 通道向前端推送事件。

use dashmap::DashMap;
use parking_lot::RwLock;
use serde::Serialize;
use std::collections::VecDeque;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

use crate::event::sse::{ProgressMeta, ProgressPhaseKind, ProgressTaskKind};

const MAX_TASK_PROGRESS_EVENTS: usize = 200;

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

/// 任务句柄
///
/// 持有取消令牌和进度信息，由 `TaskManager` 统一管理。
pub struct TaskHandle {
    pub task_id: String,
    pub cancel_token: CancellationToken,
    pub progress: Arc<RwLock<TaskProgress>>,
    pub progress_events: Arc<RwLock<VecDeque<TaskProgressEvent>>>,
    pub task_type: TaskType,
    pub project_id: Option<i64>,
    pub root_project_id: Option<i64>,
    pub source_snapshot_id: Option<i64>,
    pub task_scope: Option<String>,
}

impl TaskHandle {
    /// 取消任务
    pub fn cancel(&self) {
        self.cancel_token.cancel();
    }

    /// 检查是否已被取消
    pub fn is_cancelled(&self) -> bool {
        self.cancel_token.is_cancelled()
    }

    /// 更新进度
    pub fn update_progress(&self, current: u32, total: u32, message: &str) {
        self.update_progress_with_meta(
            current,
            total,
            message,
            Some(self.task_type.progress_meta(message)),
        );
    }

    pub fn update_progress_with_meta(
        &self,
        current: u32,
        total: u32,
        message: &str,
        meta: Option<ProgressMeta>,
    ) {
        let updated_at = chrono::Utc::now().to_rfc3339();
        let progress = TaskProgress {
            current,
            total,
            message: message.to_string(),
            meta: meta.clone(),
            updated_at: Some(updated_at.clone()),
        };
        let mut p = self.progress.write();
        *p = progress.clone();
        drop(p);

        let event = TaskProgressEvent {
            id: format!("{}-{}", self.task_id, updated_at),
            task_id: self.task_id.clone(),
            current,
            total,
            message: message.to_string(),
            percent: progress.percent() * 100.0,
            meta,
            timestamp: updated_at,
        };
        let mut events = self.progress_events.write();
        events.push_back(event);
        while events.len() > MAX_TASK_PROGRESS_EVENTS {
            events.pop_front();
        }
    }
}

/// 任务管理器
///
/// 全局注册和取消任务，管理活跃任务列表。
/// 线程安全，支持多任务并发注册。
pub struct TaskManager {
    tasks: Arc<DashMap<String, Arc<TaskHandle>>>,
}

impl TaskManager {
    /// 创建任务管理器
    pub fn new() -> Self {
        Self {
            tasks: Arc::new(DashMap::new()),
        }
    }

    /// 注册新任务
    pub fn register(&self, handle: TaskHandle) -> Arc<TaskHandle> {
        let handle = Arc::new(handle);
        {
            let progress = handle.progress.read().clone();
            let mut events = handle.progress_events.write();
            if events.is_empty() {
                let timestamp = progress
                    .updated_at
                    .clone()
                    .unwrap_or_else(|| chrono::Utc::now().to_rfc3339());
                events.push_back(TaskProgressEvent {
                    id: format!("{}-{}", handle.task_id, timestamp),
                    task_id: handle.task_id.clone(),
                    current: progress.current,
                    total: progress.total,
                    message: progress.message.clone(),
                    percent: progress.percent() * 100.0,
                    meta: progress.meta.clone(),
                    timestamp,
                });
            }
        }
        self.tasks.insert(handle.task_id.clone(), handle.clone());
        handle
    }

    /// 取消指定任务
    ///
    /// 返回 `true` 表示任务存在且已发送取消信号。
    pub fn cancel(&self, task_id: &str) -> bool {
        if let Some(handle) = self.tasks.get(task_id) {
            handle.cancel();
            true
        } else {
            false
        }
    }

    /// 列出所有活跃任务快照
    pub fn list_active_snapshots(&self) -> Vec<TaskSnapshot> {
        self.tasks
            .iter()
            .filter(|entry| !entry.value().is_cancelled())
            .map(|entry| Self::snapshot_from_handle(entry.value()))
            .collect()
    }

    /// 移除已完成的任务
    pub fn remove(&self, task_id: &str) -> Option<Arc<TaskHandle>> {
        self.tasks.remove(task_id).map(|(_, handle)| handle)
    }

    /// 活跃任务总数
    pub fn active_count(&self) -> usize {
        self.tasks
            .iter()
            .filter(|entry| !entry.value().is_cancelled())
            .count()
    }

    fn snapshot_from_handle(handle: &TaskHandle) -> TaskSnapshot {
        TaskSnapshot {
            task_id: handle.task_id.clone(),
            task_type: handle.task_type.clone(),
            project_id: handle.project_id,
            root_project_id: handle.root_project_id,
            source_snapshot_id: handle.source_snapshot_id,
            task_scope: handle.task_scope.clone(),
            progress: handle.progress.read().clone(),
            progress_events: handle.progress_events.read().iter().cloned().collect(),
            cancelled: handle.is_cancelled(),
        }
    }
}

impl Default for TaskManager {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for TaskManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TaskManager")
            .field("active_count", &self.active_count())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn galgame_task_type_uses_galgame_generation_progress_meta() {
        let meta = TaskType::GalgameGeneration.progress_meta("正在执行 Galgame 管线");

        assert_eq!(TaskType::GalgameGeneration.as_str(), "galgame_generation");
        assert!(matches!(
            &meta.task_kind,
            ProgressTaskKind::GalgameGeneration
        ));
        assert_eq!(meta.stage_code, "galgame_pipeline");
        assert_eq!(meta.stage_label, "Galgame 管线");
        assert!(matches!(&meta.phase_kind, ProgressPhaseKind::ModelWait));
        assert_eq!(meta.detail.as_deref(), Some("正在执行 Galgame 管线"));
    }
}
