//! Typed chapter-scene read and mutation commands owned by one worker.

pub use a4n_api_contract::chapter::ChapterSceneSummary;
pub use a4n_worker_contract::{WorkerEnvelope, WorkerResponse};

use serde::{Deserialize, Serialize};

pub const CHAPTER_SCENE_WORKER_ID: &str = "chapter-scene-worker";
pub const CHAPTER_SCENE_WORKER_CONTRACT_VERSION: &str = "chapter-scene-v1";
pub const CHAPTER_SCENE_LIST_OPERATION: &str = "chapter.scene.list";
pub const CHAPTER_SCENE_UPDATE_OPERATION: &str = "chapter.scene.update";
pub const CHAPTER_SCENE_DELETE_OPERATION: &str = "chapter.scene.delete";
pub const CHAPTER_SCENE_REORDER_OPERATION: &str = "chapter.scene.reorder";
pub const CHAPTER_SCENE_CAPABILITY: &str = "chapter.scene";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ChapterInput {
    pub chapter_id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct UpdateSceneInput {
    pub scene_id: i64,
    pub content: Option<String>,
    pub title: Option<String>,
    pub summary: Option<String>,
    pub scene_type: Option<String>,
    pub location: Option<String>,
    pub timeline_position: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct DeleteSceneInput {
    pub scene_id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ReorderScenesInput {
    pub order: Vec<(i64, i32)>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use a4n_worker_contract::{
        WORKER_SCHEMA_VERSION, WorkerLease, WorkerPrincipal, WorkerPrincipalKind,
    };

    #[test]
    fn scene_scope_is_stable() {
        let request = WorkerEnvelope {
            schema_version: WORKER_SCHEMA_VERSION,
            request_id: "request".into(),
            trace_id: "trace".into(),
            principal: WorkerPrincipal {
                kind: WorkerPrincipalKind::User,
                subject: "7".into(),
            },
            operation_id: CHAPTER_SCENE_LIST_OPERATION.into(),
            idempotency_key: "key".into(),
            project_id: None,
            root_project_id: None,
            target_revision: None,
            runtime_config_snapshot_ref: None,
            lease: WorkerLease {
                worker_id: CHAPTER_SCENE_WORKER_ID.into(),
                epoch: 1,
                fence_token: "control".into(),
                expires_at: "9999-12-31T23:59:59Z".into(),
            },
            payload: ChapterInput { chapter_id: 9 },
        };
        request.validate().expect("scene request validates");
    }
}
