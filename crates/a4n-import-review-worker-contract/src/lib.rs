//! Typed read commands owned by the import-review worker.

pub use a4n_api_contract::import_review::{
    CheckImportDedupInput, DedupResult, ImportProjectReviewsInput, ImportReviewSummary,
};
pub use a4n_worker_contract::{WorkerEnvelope, WorkerResponse};

pub const IMPORT_REVIEW_READ_WORKER_ID: &str = "import-review-read-worker";
pub const IMPORT_REVIEW_READ_WORKER_CONTRACT_VERSION: &str = "import-review-read-v1";
pub const IMPORT_REVIEW_LIST_OPERATION: &str = "import_review.read.list";
pub const IMPORT_REVIEW_DEDUP_OPERATION: &str = "import_review.read.dedup";
pub const IMPORT_REVIEW_READ_CAPABILITY: &str = "import_review.read";

#[cfg(test)]
mod tests {
    use super::*;
    use a4n_worker_contract::{
        WORKER_SCHEMA_VERSION, WorkerLease, WorkerPrincipal, WorkerPrincipalKind,
    };

    #[test]
    fn command_scope_is_stable() {
        let request = WorkerEnvelope {
            schema_version: WORKER_SCHEMA_VERSION,
            request_id: "request".into(),
            trace_id: "trace".into(),
            principal: WorkerPrincipal {
                kind: WorkerPrincipalKind::User,
                subject: "7".into(),
            },
            operation_id: IMPORT_REVIEW_LIST_OPERATION.into(),
            idempotency_key: "key".into(),
            project_id: Some(9),
            root_project_id: None,
            target_revision: None,
            runtime_config_snapshot_ref: None,
            lease: WorkerLease {
                worker_id: IMPORT_REVIEW_READ_WORKER_ID.into(),
                epoch: 1,
                fence_token: "control".into(),
                expires_at: "9999-12-31T23:59:59Z".into(),
            },
            payload: ImportProjectReviewsInput { project_id: 9 },
        };
        request.validate().expect("import review request validates");
    }
}
