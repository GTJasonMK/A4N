//! Typed command payloads owned by the outline records worker.

pub use a4n_api_contract::outline::{
    DeleteLatestOutlineRecordsInput, SaveChapterOutlineInput, SavePartOutlineInput,
    UpdateChapterOutlineInput,
};
pub use a4n_worker_contract::{WorkerEnvelope, WorkerResponse};

use serde::{Deserialize, Serialize};

pub const OUTLINE_WORKER_ID: &str = "outline-worker";
pub const OUTLINE_WORKER_CONTRACT_VERSION: &str = "outline-records-v2";

pub const LIST_CHAPTERS_OPERATION: &str = "outline.records.list_chapters";
pub const LIST_CHAPTER_INDEX_OPERATION: &str = "outline.records.list_chapter_index";
pub const LIST_PARTS_OPERATION: &str = "outline.records.list_parts";
pub const GENERATION_PROGRESS_OPERATION: &str = "outline.records.generation_progress";
pub const ENSURE_CHAPTER_ACCESS_OPERATION: &str = "outline.records.ensure_chapter_access";
pub const ENSURE_PART_ACCESS_OPERATION: &str = "outline.records.ensure_part_access";
pub const DELETE_CHAPTERS_OPERATION: &str = "outline.records.delete_chapters";
pub const DELETE_PARTS_OPERATION: &str = "outline.records.delete_parts";
pub const SAVE_CHAPTER_OPERATION: &str = "outline.records.save_chapter";
pub const UPDATE_CHAPTER_OPERATION: &str = "outline.records.update_chapter";
pub const SAVE_PART_OPERATION: &str = "outline.records.save_part";

pub const OUTLINE_READ_CAPABILITY: &str = "outline.records.read";
pub const OUTLINE_MUTATION_CAPABILITY: &str = "outline.records.mutation";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ProjectOutlineInput {
    pub project_id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ChapterOutlineAccessInput {
    pub project_id: i64,
    pub outline_id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PartOutlineAccessInput {
    pub project_id: i64,
    pub part_id: i64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use a4n_worker_contract::{
        WORKER_SCHEMA_VERSION, WorkerLease, WorkerPrincipal, WorkerPrincipalKind,
    };

    #[test]
    fn command_identity_and_scope_are_stable() {
        let request = WorkerEnvelope {
            schema_version: WORKER_SCHEMA_VERSION,
            request_id: "request-1".into(),
            trace_id: "trace-1".into(),
            principal: WorkerPrincipal {
                kind: WorkerPrincipalKind::User,
                subject: "7".into(),
            },
            operation_id: LIST_CHAPTERS_OPERATION.into(),
            idempotency_key: "idempotency-1".into(),
            project_id: Some(1),
            root_project_id: None,
            target_revision: None,
            runtime_config_snapshot_ref: None,
            lease: WorkerLease {
                worker_id: OUTLINE_WORKER_ID.into(),
                epoch: 1,
                fence_token: "control".into(),
                expires_at: "9999-12-31T23:59:59Z".into(),
            },
            payload: ProjectOutlineInput { project_id: 1 },
        };
        request.validate().expect("outline request should validate");
    }
}
