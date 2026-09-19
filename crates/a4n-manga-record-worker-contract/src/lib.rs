//! Typed Manga prompt read commands owned by the Manga record worker.

pub use a4n_api_contract::manga::{
    MangaChapterInput, MangaPagePromptSummary, MangaPipelineStatusSummary,
    MangaPipelineStatusesInput,
};
pub use a4n_worker_contract::{WorkerEnvelope, WorkerResponse};

pub const MANGA_RECORD_WORKER_ID: &str = "manga-record-worker";
pub const MANGA_RECORD_WORKER_CONTRACT_VERSION: &str = "manga-record-v2";
pub const MANGA_RECORD_CAPABILITY: &str = "manga.record.read";
pub const MANGA_PIPELINE_CAPABILITY: &str = "manga.pipeline.read";
pub const MANGA_RECORD_LIST_OPERATION: &str = "manga.record.list_page_prompts";
pub const MANGA_PIPELINE_STATUS_OPERATION: &str = "manga.pipeline.status";
pub const MANGA_PIPELINE_STATUSES_OPERATION: &str = "manga.pipeline.statuses";
pub const MANGA_PIPELINE_CANCEL_OPERATION: &str = "manga.pipeline.cancel";

#[cfg(test)]
mod tests {
    use super::*;
    use a4n_worker_contract::{
        WORKER_SCHEMA_VERSION, WorkerLease, WorkerPrincipal, WorkerPrincipalKind,
    };

    #[test]
    fn manga_record_scope_is_stable() {
        let request = WorkerEnvelope {
            schema_version: WORKER_SCHEMA_VERSION,
            request_id: "request".into(),
            trace_id: "trace".into(),
            principal: WorkerPrincipal {
                kind: WorkerPrincipalKind::User,
                subject: "7".into(),
            },
            operation_id: MANGA_RECORD_LIST_OPERATION.into(),
            idempotency_key: "key".into(),
            project_id: Some(9),
            root_project_id: None,
            target_revision: None,
            runtime_config_snapshot_ref: None,
            lease: WorkerLease {
                worker_id: MANGA_RECORD_WORKER_ID.into(),
                epoch: 1,
                fence_token: "control".into(),
                expires_at: "9999-12-31T23:59:59Z".into(),
            },
            payload: MangaChapterInput {
                project_id: 9,
                chapter_id: 10,
            },
        };
        request.validate().expect("manga record request validates");
    }
}
