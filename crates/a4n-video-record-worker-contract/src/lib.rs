//! Typed reads for persisted generated-video records.

pub use a4n_api_contract::image::GeneratedImageSummary;
pub use a4n_api_contract::video::{
    ChapterVideoPromptSummary, GeneratedVideoSummary, VideoPromptReadApplicationPort,
    VideoRecordReadApplicationPort,
};
pub use a4n_worker_contract::{WorkerEnvelope, WorkerResponse};

use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const VIDEO_RECORD_WORKER_ID: &str = "video-record-worker";
pub const VIDEO_RECORD_WORKER_CONTRACT_VERSION: &str = "video-record-v2";
pub const VIDEO_RECORD_CAPABILITY: &str = "video.record.read";
pub const VIDEO_PROMPT_CAPABILITY: &str = "video.prompt.read";
pub const VIDEO_RECORD_PLAYBACK_OPERATION: &str = "video.record.playback";
pub const VIDEO_RECORD_LIST_OPERATION: &str = "video.record.list_by_chapter";
pub const VIDEO_RECORD_REFERENCE_IMAGES_OPERATION: &str = "video.record.reference_images";
pub const VIDEO_PROMPT_LIST_OPERATION: &str = "video.prompt.list_by_chapter";
pub const VIDEO_PROMPT_VALIDATE_SOURCE_OPERATION: &str = "video.prompt.validate_source";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct VideoIdInput {
    pub video_id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ChapterVideosInput {
    pub project_id: i64,
    pub chapter_id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ReferenceImagesInput {
    pub project_id: i64,
    pub chapter_id: i64,
    pub reference_image_ids_json: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct VideoSourceInput {
    pub project_id: i64,
    pub chapter_id: i64,
    pub operation: String,
    pub writable: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use a4n_worker_contract::{
        WORKER_SCHEMA_VERSION, WorkerLease, WorkerPrincipal, WorkerPrincipalKind,
    };

    #[test]
    fn video_record_scope_is_stable() {
        let request = WorkerEnvelope {
            schema_version: WORKER_SCHEMA_VERSION,
            request_id: "request".into(),
            trace_id: "trace".into(),
            principal: WorkerPrincipal {
                kind: WorkerPrincipalKind::User,
                subject: "7".into(),
            },
            operation_id: VIDEO_RECORD_LIST_OPERATION.into(),
            idempotency_key: "key".into(),
            project_id: Some(9),
            root_project_id: None,
            target_revision: None,
            runtime_config_snapshot_ref: None,
            lease: WorkerLease {
                worker_id: VIDEO_RECORD_WORKER_ID.into(),
                epoch: 1,
                fence_token: "control".into(),
                expires_at: "9999-12-31T23:59:59Z".into(),
            },
            payload: ChapterVideosInput {
                project_id: 9,
                chapter_id: 10,
            },
        };
        request.validate().expect("video record request validates");
    }
}
