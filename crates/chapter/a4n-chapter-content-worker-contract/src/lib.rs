//! Typed chapter-content commands owned by the chapter-content worker.

pub use a4n_api_contract::chapter::{
    ChapterSummary, ImportChapterInput, UpdateChapterContentInput,
};
pub use a4n_worker_contract::{WorkerEnvelope, WorkerResponse};

use serde::{Deserialize, Serialize};

pub const CHAPTER_CONTENT_WORKER_ID: &str = "chapter-content-worker";
pub const CHAPTER_CONTENT_WORKER_CONTRACT_VERSION: &str = "chapter-content-v2";
pub const CHAPTER_CONTENT_LIST_OPERATION: &str = "chapter.content.list";
pub const CHAPTER_CONTENT_SUMMARY_OPERATION: &str = "chapter.content.summary";
pub const CHAPTER_CONTENT_UPDATE_OPERATION: &str = "chapter.content.update";
pub const CHAPTER_CONTENT_IMPORT_OPERATION: &str = "chapter.content.import";
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ChapterContentMutationResult {
    pub chapter_id: i64,
    pub workflow_run_id: i64,
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

    #[test]
    fn mutation_operations_validate_with_their_wire_scope() {
        let lease = WorkerLease {
            worker_id: CHAPTER_CONTENT_WORKER_ID.into(),
            epoch: 1,
            fence_token: "control".into(),
            expires_at: "9999-12-31T23:59:59Z".into(),
        };
        let update = WorkerEnvelope {
            schema_version: WORKER_SCHEMA_VERSION,
            request_id: "update-request".into(),
            trace_id: "update-trace".into(),
            principal: WorkerPrincipal {
                kind: WorkerPrincipalKind::User,
                subject: "7".into(),
            },
            operation_id: CHAPTER_CONTENT_UPDATE_OPERATION.into(),
            idempotency_key: "update-key".into(),
            project_id: None,
            root_project_id: None,
            target_revision: None,
            runtime_config_snapshot_ref: None,
            lease: lease.clone(),
            payload: UpdateChapterContentInput {
                chapter_id: 9,
                content: "新正文".into(),
                title: None,
            },
        };
        update.validate().expect("chapter update validates");

        let import = WorkerEnvelope {
            schema_version: WORKER_SCHEMA_VERSION,
            request_id: "import-request".into(),
            trace_id: "import-trace".into(),
            principal: WorkerPrincipal {
                kind: WorkerPrincipalKind::User,
                subject: "7".into(),
            },
            operation_id: CHAPTER_CONTENT_IMPORT_OPERATION.into(),
            idempotency_key: "import-key".into(),
            project_id: Some(9),
            root_project_id: None,
            target_revision: None,
            runtime_config_snapshot_ref: None,
            lease,
            payload: ImportChapterInput {
                project_id: 9,
                chapter_number: 3,
                content: "导入正文".into(),
                title: Some("第三章".into()),
            },
        };
        import.validate().expect("chapter import validates");
    }
}
