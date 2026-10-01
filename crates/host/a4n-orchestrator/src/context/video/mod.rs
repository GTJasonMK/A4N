//! Video 领域私有 owner。
//!
//! 持有 `DbHandle`，封装视频提示词、生成图/视频记录与素材库相关数据库行为。
//! LLM/预览/流水线编排仍在 `video/` 模块，但 DB 只经本 owner。

use a4n_db::DbHandle;
use a4n_db::generation::{
    RecordCompletedGenerationRunWithCheckpointInput, RecordStartedGenerationRunInput,
};
use a4n_db::video::ChapterVideoPromptTimelineRecord;
use a4n_db_models::models::{chapter::chapter_video_timeline, media::image_publication};
use a4n_domain::media::{
    CreateGeneratedImageRecordDraftInput, CreateGeneratedVideoRecordDraftInput,
    CreateImagePublicationIntentInput, GeneratedImageRecord, GeneratedImageReferenceRecord,
    GeneratedVideoErrorStatusBatchInput, GeneratedVideoRecord, ImagePublicationFileFacts,
    UpdateGeneratedImageRecordInput, UpdateGeneratedVideoRecordInput,
};
use a4n_domain::source_text_hash as source_content_sha256;
use a4n_domain::video_prompt::{ChapterVideoPromptPatch, VideoPromptDownstreamDependencies};
use a4n_domain::video_timeline::{NativeVideoTimelineKey, VideoTimelineGap};
use a4n_error::AppError;
use a4n_video::validate_video_max_concurrent;
use a4n_video_contract::VideoSegmentExecutionRecipeV1;
use sea_orm::DatabaseConnection;
use serde_json::{Value, json};
use std::collections::BTreeSet;
use std::sync::Arc;
use tokio::sync::{Mutex, OwnedMutexGuard};

use super::OrchestratorContext;
use crate::video::video_db_error;
use crate::workflow::{
    UpsertWorkflowCheckpointInput, WORKFLOW_STATUS_CANCELLED, WORKFLOW_STATUS_FAILED,
    WORKFLOW_STATUS_INTERRUPTED, WORKFLOW_STATUS_RUNNING, WORKFLOW_STATUS_SUPERSEDED,
    WorkflowProgressEventInput, WorkflowRunRecoveryRecord,
};

fn terminal_workflow_status_label(status: &str) -> &'static str {
    match status {
        WORKFLOW_STATUS_CANCELLED => "取消",
        WORKFLOW_STATUS_FAILED => "失败",
        WORKFLOW_STATUS_SUPERSEDED => "被取代",
        _ => "结束",
    }
}

/// Video 领域 application owner。
#[derive(Clone)]
pub(super) struct VideoApplication {
    database: DbHandle,
    // ponytail: serialize publication file/DB commits within the sole Host; shard if throughput requires it.
    image_publication_gate: Arc<Mutex<()>>,
}

impl VideoApplication {
    pub(super) fn new(database: DbHandle) -> Self {
        Self {
            database,
            image_publication_gate: Arc::new(Mutex::new(())),
        }
    }

    fn connection(&self) -> &DatabaseConnection {
        self.database.connection()
    }
}

mod application;
mod facade;
