//! Typed HTTP adapter for the novel-planning worker.

use a4n_api_contract::chapter_visual_plan::{
    ChapterVisualPlanReadResult, ChapterVisualPlanRecordInput,
    ChapterVisualPlanRecordPersistenceApplicationPort, ChapterVisualPlanRecordReadApplicationPort,
    ChapterVisualPlanSetView, ChapterVisualPlanSourceInput, ChapterVisualPlanValidationResult,
    ChapterVisualPlanView, ConfirmedChapterVisualPlanView, ReadChapterVisualPlanInput,
    SaveChapterVisualPlanDraftInput, ValidateChapterVisualPlanInput,
};
use a4n_api_contract::shooting_script::{
    ChapterShootingScriptSourceInput, ChapterShootingScriptView, DeleteChapterShootingScriptInput,
    EditChapterShootingScriptInput, EditedChapterShootingScriptView,
    PublishChapterShootingScriptInput, ShootingScriptCurrentReadApplicationPort,
    ShootingScriptRecordMutationApplicationPort, ShootingScriptUnitProgressReadApplicationPort,
};
use a4n_error::AppError;
use a4n_novel_planning_worker_contract::{
    CONFIRM_VISUAL_PLAN_OPERATION, ConfirmVisualPlanPayload, CurrentShootingScriptPayload,
    DELETE_SHOOTING_SCRIPT_OPERATION, DISCARD_VISUAL_PLAN_OPERATION, DeleteShootingScriptPayload,
    DiscardVisualPlanPayload, EDIT_SHOOTING_SCRIPT_OPERATION, EditShootingScriptPayload,
    GET_CURRENT_SHOOTING_SCRIPT_OPERATION, GET_SHOOTING_SCRIPT_UNIT_PROGRESS_OPERATION,
    GET_VISUAL_PLAN_OPERATION, GetVisualPlanPayload, NOVEL_PLANNING_WORKER_ID,
    PUBLISH_SHOOTING_SCRIPT_OPERATION, PublishShootingScriptPayload, READ_VISUAL_PLAN_OPERATION,
    ReadVisualPlanPayload, SAVE_VISUAL_PLAN_DRAFT_OPERATION, SaveVisualPlanDraftPayload,
    ShootingScriptUnitProgressPayload, VALIDATE_VISUAL_PLAN_OPERATION, ValidateVisualPlanPayload,
};
use a4n_worker_client::{HttpWorkerClient, WorkerClientError};
use a4n_worker_contract::{
    WORKER_SCHEMA_VERSION, WorkerEnvelope, WorkerError, WorkerErrorCode, WorkerLease,
    WorkerOperationStatus, WorkerPrincipal, WorkerPrincipalKind, WorkerResponse,
};

const DEFAULT_WORKER_URL: &str = "http://127.0.0.1:47651";
const CONTROL_LEASE_FENCE: &str = "control-request";
const CONTROL_LEASE_EXPIRY: &str = "9999-12-31T23:59:59Z";

#[derive(Clone)]
pub struct NovelPlanningWorkerClient {
    transport: HttpWorkerClient,
}

impl std::fmt::Debug for NovelPlanningWorkerClient {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("NovelPlanningWorkerClient")
            .field("transport", &self.transport)
            .finish()
    }
}

impl NovelPlanningWorkerClient {
    pub fn new(base_url: impl Into<String>, auth_token: Option<String>) -> Self {
        let transport = HttpWorkerClient::new(base_url, NOVEL_PLANNING_WORKER_ID);
        let transport = match auth_token {
            Some(token) => transport.with_auth_token(token),
            None => transport,
        };
        Self { transport }
    }

    pub fn from_env() -> Self {
        let url = std::env::var("A4N_NOVEL_PLANNING_WORKER_URL")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| DEFAULT_WORKER_URL.to_string());
        let token = std::env::var("A4N_NOVEL_PLANNING_WORKER_TOKEN")
            .ok()
            .filter(|value| !value.trim().is_empty());
        Self::new(url, token)
    }

    async fn command<I, O>(
        &self,
        user_id: i64,
        project_id: i64,
        target_revision: Option<i32>,
        operation_id: &str,
        payload: I,
    ) -> Result<O, AppError>
    where
        I: serde::Serialize,
        O: serde::de::DeserializeOwned,
    {
        if user_id <= 0 || project_id <= 0 {
            return Err(AppError::InvalidArgument(
                "user_id 和 project_id 必须大于 0".into(),
            ));
        }
        if target_revision.is_some_and(|revision| revision <= 0) {
            return Err(AppError::InvalidArgument(
                "expected_revision 必须大于 0".into(),
            ));
        }
        let request_id = uuid::Uuid::new_v4().to_string();
        let request = WorkerEnvelope {
            schema_version: WORKER_SCHEMA_VERSION,
            request_id: request_id.clone(),
            trace_id: request_id,
            principal: WorkerPrincipal {
                kind: WorkerPrincipalKind::User,
                subject: user_id.to_string(),
            },
            operation_id: operation_id.to_string(),
            idempotency_key: uuid::Uuid::new_v4().to_string(),
            project_id: Some(project_id),
            root_project_id: None,
            target_revision: target_revision.map(|revision| revision as u64),
            runtime_config_snapshot_ref: None,
            lease: WorkerLease {
                worker_id: NOVEL_PLANNING_WORKER_ID.into(),
                epoch: 1,
                fence_token: CONTROL_LEASE_FENCE.into(),
                expires_at: CONTROL_LEASE_EXPIRY.into(),
            },
            payload,
        };
        let response: WorkerResponse<O> = self
            .transport
            .command(operation_id, request)
            .await
            .map_err(worker_client_error)?;
        match response.status {
            WorkerOperationStatus::Completed => response.payload.ok_or_else(|| {
                AppError::ConfigError(format!(
                    "novel-planning worker 完成响应缺少 payload: {operation_id}"
                ))
            }),
            WorkerOperationStatus::Failed => Err(worker_response_error(
                response.error.as_ref().expect("validated failed response"),
                operation_id,
            )),
            status => Err(AppError::ConfigError(format!(
                "novel-planning worker 返回未完成状态: operation={operation_id}, status={status:?}"
            ))),
        }
    }
}

#[async_trait::async_trait]
impl ShootingScriptCurrentReadApplicationPort for NovelPlanningWorkerClient {
    async fn current(
        &self,
        user_id: i64,
        input: ChapterShootingScriptSourceInput,
    ) -> Result<Option<ChapterShootingScriptView>, AppError> {
        self.command::<CurrentShootingScriptPayload, Option<ChapterShootingScriptView>>(
            user_id,
            input.project_id,
            None,
            GET_CURRENT_SHOOTING_SCRIPT_OPERATION,
            input,
        )
        .await
    }
}

#[async_trait::async_trait]
impl ShootingScriptUnitProgressReadApplicationPort for NovelPlanningWorkerClient {
    async fn unit_progress(
        &self,
        user_id: i64,
        input: ChapterShootingScriptSourceInput,
    ) -> Result<
        Option<a4n_api_contract::shooting_script::ChapterShootingScriptUnitProgressView>,
        AppError,
    > {
        self.command::<
            ShootingScriptUnitProgressPayload,
            Option<a4n_api_contract::shooting_script::ChapterShootingScriptUnitProgressView>,
        >(
            user_id,
            input.project_id,
            None,
            GET_SHOOTING_SCRIPT_UNIT_PROGRESS_OPERATION,
            input,
        )
        .await
    }
}

#[async_trait::async_trait]
impl ShootingScriptRecordMutationApplicationPort for NovelPlanningWorkerClient {
    async fn publish(
        &self,
        user_id: i64,
        input: PublishChapterShootingScriptInput,
    ) -> Result<ChapterShootingScriptView, AppError> {
        self.command::<PublishShootingScriptPayload, ChapterShootingScriptView>(
            user_id,
            input.project_id,
            input.expected_revision,
            PUBLISH_SHOOTING_SCRIPT_OPERATION,
            input,
        )
        .await
    }

    async fn edit(
        &self,
        user_id: i64,
        input: EditChapterShootingScriptInput,
    ) -> Result<EditedChapterShootingScriptView, AppError> {
        self.command::<EditShootingScriptPayload, EditedChapterShootingScriptView>(
            user_id,
            input.project_id,
            Some(input.expected_revision),
            EDIT_SHOOTING_SCRIPT_OPERATION,
            input,
        )
        .await
    }

    async fn delete(
        &self,
        user_id: i64,
        input: DeleteChapterShootingScriptInput,
    ) -> Result<(), AppError> {
        self.command::<DeleteShootingScriptPayload, ()>(
            user_id,
            input.project_id,
            Some(input.expected_revision),
            DELETE_SHOOTING_SCRIPT_OPERATION,
            input,
        )
        .await
    }
}

#[async_trait::async_trait]
impl ChapterVisualPlanRecordPersistenceApplicationPort for NovelPlanningWorkerClient {
    async fn save_draft(
        &self,
        user_id: i64,
        input: SaveChapterVisualPlanDraftInput,
    ) -> Result<ChapterVisualPlanView, AppError> {
        self.command::<SaveVisualPlanDraftPayload, ChapterVisualPlanView>(
            user_id,
            input.project_id,
            input.expected_revision,
            SAVE_VISUAL_PLAN_DRAFT_OPERATION,
            input,
        )
        .await
    }

    async fn confirm(
        &self,
        user_id: i64,
        input: ChapterVisualPlanRecordInput,
    ) -> Result<ConfirmedChapterVisualPlanView, AppError> {
        self.command::<ConfirmVisualPlanPayload, ConfirmedChapterVisualPlanView>(
            user_id,
            input.project_id,
            Some(input.expected_revision),
            CONFIRM_VISUAL_PLAN_OPERATION,
            input,
        )
        .await
    }

    async fn discard(
        &self,
        user_id: i64,
        input: ChapterVisualPlanRecordInput,
    ) -> Result<(), AppError> {
        self.command::<DiscardVisualPlanPayload, ()>(
            user_id,
            input.project_id,
            Some(input.expected_revision),
            DISCARD_VISUAL_PLAN_OPERATION,
            input,
        )
        .await
    }
}

#[async_trait::async_trait]
impl ChapterVisualPlanRecordReadApplicationPort for NovelPlanningWorkerClient {
    async fn get(
        &self,
        user_id: i64,
        input: ChapterVisualPlanSourceInput,
    ) -> Result<ChapterVisualPlanSetView, AppError> {
        self.command::<GetVisualPlanPayload, ChapterVisualPlanSetView>(
            user_id,
            input.project_id,
            None,
            GET_VISUAL_PLAN_OPERATION,
            input,
        )
        .await
    }

    async fn read(
        &self,
        user_id: i64,
        input: ReadChapterVisualPlanInput,
    ) -> Result<ChapterVisualPlanReadResult, AppError> {
        self.command::<ReadVisualPlanPayload, ChapterVisualPlanReadResult>(
            user_id,
            input.project_id,
            input.expected_revision,
            READ_VISUAL_PLAN_OPERATION,
            input,
        )
        .await
    }

    async fn validate(
        &self,
        user_id: i64,
        input: ValidateChapterVisualPlanInput,
    ) -> Result<ChapterVisualPlanValidationResult, AppError> {
        self.command::<ValidateVisualPlanPayload, ChapterVisualPlanValidationResult>(
            user_id,
            input.project_id,
            Some(input.expected_revision),
            VALIDATE_VISUAL_PLAN_OPERATION,
            input,
        )
        .await
    }
}

fn worker_client_error(error: WorkerClientError) -> AppError {
    AppError::ConfigError(format!("novel-planning worker 请求失败: {error}"))
}

fn worker_response_error(error: &WorkerError, operation: &str) -> AppError {
    match error.code {
        WorkerErrorCode::InvalidRequest => AppError::InvalidArgument(error.message.clone()),
        WorkerErrorCode::Unauthorized => AppError::Forbidden(error.message.clone()),
        WorkerErrorCode::NotFound => AppError::NotFound(error.message.clone()),
        WorkerErrorCode::Conflict => AppError::InvalidState {
            current: error.message.clone(),
            operation: operation.to_string(),
        },
        WorkerErrorCode::LeaseLost
        | WorkerErrorCode::Unavailable
        | WorkerErrorCode::Timeout
        | WorkerErrorCode::VersionMismatch
        | WorkerErrorCode::Internal
        | WorkerErrorCode::Cancelled => AppError::ConfigError(format!(
            "novel-planning worker {operation} 失败: {}",
            error.message
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use a4n_worker_contract::{WorkerCapabilities, WorkerHealth, WorkerHealthStatus};

    #[test]
    fn client_debug_does_not_expose_worker_url_or_token() {
        let client = NovelPlanningWorkerClient::new(
            "https://user:secret@worker.example.test",
            Some("worker-token".into()),
        );
        let debug = format!("{client:?}");
        assert!(!debug.contains("worker.example.test"));
        assert!(!debug.contains("worker-token"));
    }

    #[test]
    fn failed_worker_errors_keep_revision_conflict_semantics() {
        let error = WorkerError {
            code: WorkerErrorCode::Conflict,
            message: "shooting_script_revision_conflict:id=4,expected=2".into(),
            retryable: false,
        };
        assert!(matches!(
            worker_response_error(&error, EDIT_SHOOTING_SCRIPT_OPERATION),
            AppError::InvalidState { .. }
        ));
    }

    #[allow(dead_code)]
    fn _shared_health_types_compile(health: WorkerHealth, capabilities: WorkerCapabilities) {
        assert_eq!(health.status, WorkerHealthStatus::Healthy);
        assert!(capabilities.capabilities.is_empty());
    }
}
