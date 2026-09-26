//! PostgreSQL-backed shooting-script CRUD worker.

use std::{env, net::SocketAddr, sync::Arc};

use a4n_api_contract::chapter_visual_plan::{
    ChapterVisualPlanReadResult, ChapterVisualPlanRecordInput, ChapterVisualPlanRecordSummary,
    ChapterVisualPlanSetView, ChapterVisualPlanSourceInput, ChapterVisualPlanValidationResult,
    ChapterVisualPlanView, ConfirmedChapterVisualPlanView, ReadChapterVisualPlanInput,
    SaveChapterVisualPlanDraftInput, ValidateChapterVisualPlanInput, VisualPlanProjection,
    VisualPlanReadView,
};
use a4n_api_contract::shooting_script::{
    ChapterShootingScriptRecordSummary, ChapterShootingScriptSourceInput,
    ChapterShootingScriptUnitProgressView, ChapterShootingScriptView,
    DeleteChapterShootingScriptInput, EditChapterShootingScriptInput,
    EditChapterShootingScriptProgressInput, EditedChapterShootingScriptProgressView,
    EditedChapterShootingScriptView, PublishChapterShootingScriptInput,
};
#[cfg(test)]
use a4n_db::DatabaseConnection;
use a4n_db::{DbHandle, chapter_visual_plan::ChapterVisualPlanDbError};
use a4n_domain::collaboration::ProjectAccessRole;
use a4n_novel_planning_worker_contract::{
    CONFIRM_VISUAL_PLAN_OPERATION, DELETE_SHOOTING_SCRIPT_OPERATION, DISCARD_VISUAL_PLAN_OPERATION,
    EDIT_SHOOTING_SCRIPT_OPERATION, EDIT_SHOOTING_SCRIPT_PROGRESS_OPERATION,
    GET_CURRENT_SHOOTING_SCRIPT_OPERATION, GET_SHOOTING_SCRIPT_UNIT_PROGRESS_OPERATION,
    GET_VISUAL_PLAN_OPERATION, NOVEL_PLANNING_WORKER_CONTRACT_VERSION, NOVEL_PLANNING_WORKER_ID,
    OptionalWorkerPayload, PUBLISH_SHOOTING_SCRIPT_OPERATION, READ_VISUAL_PLAN_OPERATION,
    SAVE_VISUAL_PLAN_DRAFT_OPERATION, SHOOTING_SCRIPT_CURRENT_CAPABILITY,
    SHOOTING_SCRIPT_DELETE_CAPABILITY, SHOOTING_SCRIPT_EDIT_CAPABILITY,
    SHOOTING_SCRIPT_EDIT_PROGRESS_CAPABILITY, SHOOTING_SCRIPT_PUBLISH_CAPABILITY,
    SHOOTING_SCRIPT_UNIT_PROGRESS_CAPABILITY, VALIDATE_VISUAL_PLAN_OPERATION,
    VISUAL_PLAN_PERSISTENCE_CAPABILITY,
};
use a4n_worker_client::WORKER_AUTH_HEADER;
use a4n_worker_contract::{
    WORKER_CAPABILITIES_PATH, WORKER_HEALTH_PATH, WORKER_SCHEMA_VERSION, WorkerCapabilities,
    WorkerCapability, WorkerEnvelope, WorkerError, WorkerErrorCode, WorkerHealth,
    WorkerHealthStatus, WorkerOperationRef, WorkerOperationStatus, WorkerResponse,
};
use axum::{
    Json, Router,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use subtle::ConstantTimeEq;

const WORKER_ADDR_ENV: &str = "A4N_NOVEL_PLANNING_WORKER_ADDR";
const DATABASE_URL_ENV: &str = "A4N_DATABASE_URL";
const WORKER_TOKEN_ENV: &str = "A4N_NOVEL_PLANNING_WORKER_TOKEN";
const DEFAULT_ADDR: &str = "127.0.0.1:47651";

#[derive(Clone)]
pub struct NovelPlanningWorker {
    state: WorkerState,
}

#[derive(Clone)]
struct WorkerState {
    database: DbHandle,
    worker_id: Arc<str>,
    auth_token: Arc<str>,
}

#[derive(Debug)]
struct WorkerFailure {
    code: WorkerErrorCode,
    message: String,
    retryable: bool,
}

impl NovelPlanningWorker {
    pub fn new(
        database: a4n_db::DbHandle,
        worker_id: impl Into<String>,
        auth_token: impl Into<String>,
    ) -> Self {
        Self {
            state: WorkerState {
                database,
                worker_id: Arc::<str>::from(worker_id.into()),
                auth_token: Arc::<str>::from(auth_token.into()),
            },
        }
    }

    pub fn router(&self) -> Router {
        Router::new()
            .route(WORKER_HEALTH_PATH, get(health))
            .route(WORKER_CAPABILITIES_PATH, get(capabilities))
            .route(
                "/v1/commands/novel_planning.shooting_script.current",
                post(current),
            )
            .route(
                "/v1/commands/novel_planning.shooting_script.unit_progress",
                post(unit_progress),
            )
            .route(
                "/v1/commands/novel_planning.shooting_script.edit",
                post(edit),
            )
            .route(
                "/v1/commands/novel_planning.shooting_script.edit_progress",
                post(edit_progress),
            )
            .route(
                "/v1/commands/novel_planning.shooting_script.delete",
                post(delete),
            )
            .route(
                "/v1/commands/novel_planning.shooting_script.publish",
                post(publish),
            )
            .route(
                "/v1/commands/novel_planning.visual_plan.save_draft",
                post(save_visual_plan_draft),
            )
            .route(
                "/v1/commands/novel_planning.visual_plan.confirm",
                post(confirm_visual_plan),
            )
            .route(
                "/v1/commands/novel_planning.visual_plan.discard",
                post(discard_visual_plan),
            )
            .route(
                "/v1/commands/novel_planning.visual_plan.get",
                post(get_visual_plan),
            )
            .route(
                "/v1/commands/novel_planning.visual_plan.read",
                post(read_visual_plan),
            )
            .route(
                "/v1/commands/novel_planning.visual_plan.validate",
                post(validate_visual_plan),
            )
            .with_state(self.state.clone())
    }

    pub async fn serve(self, address: SocketAddr) -> anyhow::Result<()> {
        let listener = tokio::net::TcpListener::bind(address).await?;
        axum::serve(listener, self.router())
            .with_graceful_shutdown(shutdown_signal())
            .await
            .map_err(anyhow::Error::from)
    }
}

include!("worker/runtime.rs");
include!("worker/routes.rs");
include!("worker/state.rs");
include!("worker/helpers.rs");

#[cfg(test)]
mod tests {
    use super::*;
    use a4n_worker_contract::WorkerResponse;
    use axum::{
        body::Body,
        http::{Request, header},
    };
    use serde_json::json;
    use tower::ServiceExt;

    fn worker() -> NovelPlanningWorker {
        NovelPlanningWorker::new(
            DbHandle::new(Arc::new(DatabaseConnection::Disconnected)),
            NOVEL_PLANNING_WORKER_ID,
            "worker-token",
        )
    }

    #[tokio::test]
    async fn health_and_capabilities_require_authentication() {
        let router = worker().router();
        for path in [WORKER_HEALTH_PATH, WORKER_CAPABILITIES_PATH] {
            let response = router
                .clone()
                .oneshot(Request::get(path).body(Body::empty()).unwrap())
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        }
    }

    #[tokio::test]
    async fn current_command_rejects_operation_mismatch_before_database_access() {
        let request = json!({
            "schema_version": WORKER_SCHEMA_VERSION,
            "request_id": "request-1",
            "trace_id": "trace-1",
            "principal": {"kind": "user", "subject": "7"},
            "operation_id": "wrong.operation",
            "idempotency_key": "idempotency-1",
            "project_id": 99,
            "root_project_id": null,
            "target_revision": null,
            "runtime_config_snapshot_ref": null,
            "lease": {
                "worker_id": NOVEL_PLANNING_WORKER_ID,
                "epoch": 1,
                "fence_token": "fence-1",
                "expires_at": "2099-01-01T00:00:00Z"
            },
            "payload": {"project_id": 1, "source_chapter_id": 2, "visual_plan_id": null}
        });
        let response = worker()
            .router()
            .oneshot(
                Request::post("/v1/commands/novel_planning.shooting_script.current")
                    .header(WORKER_AUTH_HEADER, "worker-token")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(serde_json::to_vec(&request).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let response: WorkerResponse<Option<ChapterShootingScriptView>> =
            serde_json::from_slice(&body).unwrap();
        response
            .validate()
            .expect("worker failure response contract");
        assert_eq!(response.status, WorkerOperationStatus::Failed);
        assert_eq!(
            response.error.as_ref().map(|error| error.code),
            Some(WorkerErrorCode::InvalidRequest)
        );
    }

    #[test]
    fn worker_address_rejects_public_bindings() {
        assert!(worker_addr_from_value("0.0.0.0:47651").is_err());
        assert!(worker_addr_from_value("127.0.0.1:0").is_err());
        assert!(worker_addr_from_value("127.0.0.1:47651").is_ok());
    }

    fn worker_addr_from_value(value: &str) -> anyhow::Result<SocketAddr> {
        let address: SocketAddr = value.parse()?;
        if !address.ip().is_loopback() || address.port() == 0 {
            anyhow::bail!("invalid worker address")
        }
        Ok(address)
    }

    #[test]
    fn revision_conflict_preserves_explicit_error_code() {
        let failure = shooting_script_error(
            a4n_db::shooting_script::ShootingScriptDbError::RevisionConflict {
                record_id: 4,
                expected_revision: 2,
            },
        );
        assert_eq!(failure.code, WorkerErrorCode::Conflict);
        assert!(
            failure
                .message
                .contains("shooting_script_revision_conflict")
        );
    }
}
