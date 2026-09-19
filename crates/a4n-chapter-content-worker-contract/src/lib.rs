//! Typed read commands owned by the chapter-content worker.

pub use a4n_api_contract::chapter::ChapterSummary;
pub use a4n_worker_contract::{WorkerEnvelope, WorkerResponse};

use serde::{Deserialize, Serialize};

pub const CHAPTER_CONTENT_WORKER_ID: &str = "chapter-content-worker";
pub const CHAPTER_CONTENT_WORKER_CONTRACT_VERSION: &str = "chapter-content-v1";
pub const CHAPTER_CONTENT_LIST_OPERATION: &str = "chapter.content.list";
pub const CHAPTER_CONTENT_SUMMARY_OPERATION: &str = "chapter.content.summary";
pub const CHAPTER_CONTENT_CAPABILITY: &str = "chapter.content";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ProjectInput {
    pub project_id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ChapterInput {
    pub chapter_id: i64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use a4n_worker_contract::{
        WORKER_SCHEMA_VERSION, WorkerLease, WorkerPrincipal, WorkerPrincipalKind,
    };

    #[test]
    fn chapter_content_scope_is_stable() {
        let request = WorkerEnvelope {
            schema_version: WORKER_SCHEMA_VERSION,
            request_id: "request".into(),
            trace_id: "trace".into(),
            principal: WorkerPrincipal {
                kind: WorkerPrincipalKind::User,
                subject: "7".into(),
            },
            operation_id: CHAPTER_CONTENT_SUMMARY_OPERATION.into(),
            idempotency_key: "key".into(),
            project_id: Some(9),
            root_project_id: None,
            target_revision: None,
            runtime_config_snapshot_ref: None,
            lease: WorkerLease {
                worker_id: CHAPTER_CONTENT_WORKER_ID.into(),
                epoch: 1,
                fence_token: "control".into(),
                expires_at: "9999-12-31T23:59:59Z".into(),
            },
            payload: ChapterInput { chapter_id: 9 },
        };
        request
            .validate()
            .expect("chapter content request validates");
    }
}
