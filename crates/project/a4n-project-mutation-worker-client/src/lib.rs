//! Typed Host adapter for the project-mutation worker.

use a4n_api_contract::project::{
    CreateDerivedProjectInput, CreateFreeformVideoProjectInput, CreateProjectInput,
    ProjectMutationApplicationPort, ProjectSummary, UpdateProjectInput,
};
use a4n_error::AppError;
use a4n_project_mutation_worker_contract::{
    CreateDerivedProjectInput as WorkerCreateDerivedProjectInput,
    CreateFreeformVideoProjectInput as WorkerCreateFreeformVideoProjectInput,
    CreateProjectInput as WorkerCreateProjectInput, PROJECT_MUTATION_CREATE_DERIVED_OPERATION,
    PROJECT_MUTATION_CREATE_FREEFORM_VIDEO_OPERATION, PROJECT_MUTATION_CREATE_OPERATION,
    PROJECT_MUTATION_UPDATE_OPERATION, PROJECT_MUTATION_WORKER_ID,
    UpdateProjectInput as WorkerUpdateProjectInput,
};
use a4n_worker_client::{HttpWorkerClient, WorkerClientError};
use a4n_worker_contract::{
    WORKER_SCHEMA_VERSION, WorkerEnvelope, WorkerError, WorkerErrorCode, WorkerLease,
    WorkerOperationStatus, WorkerPrincipal, WorkerPrincipalKind, WorkerResponse,
};

const DEFAULT_WORKER_URL: &str = "http://127.0.0.1:47669";
const CONTROL_LEASE_FENCE: &str = "control-request";
const CONTROL_LEASE_EXPIRY: &str = "9999-12-31T23:59:59Z";

#[derive(Clone)]
pub struct ProjectMutationWorkerClient {
    transport: HttpWorkerClient,
}

impl std::fmt::Debug for ProjectMutationWorkerClient {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ProjectMutationWorkerClient")
            .field("transport", &self.transport)
            .finish()
    }
}

impl ProjectMutationWorkerClient {
    pub fn new(base_url: impl Into<String>, auth_token: Option<String>) -> Self {
        let transport = HttpWorkerClient::new(base_url, PROJECT_MUTATION_WORKER_ID);
        let transport = match auth_token {
            Some(token) => transport.with_auth_token(token),
            None => transport,
        };
        Self { transport }
    }

    pub fn from_env() -> Self {
        let url = std::env::var("A4N_PROJECT_MUTATION_WORKER_URL")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| DEFAULT_WORKER_URL.to_string());
        let token = std::env::var("A4N_PROJECT_MUTATION_WORKER_TOKEN")
            .ok()
            .filter(|value| !value.trim().is_empty());
        Self::new(url, token)
    }

    async fn command<I, O>(
        &self,
        user_id: i64,
        project_id: Option<i64>,
        operation: &str,
        payload: I,
    ) -> Result<O, AppError>
    where
        I: serde::Serialize,
        O: serde::de::DeserializeOwned,
    {
        if user_id <= 0 {
            return Err(AppError::InvalidArgument("user_id 必须大于 0".into()));
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
            operation_id: operation.into(),
            idempotency_key: uuid::Uuid::new_v4().to_string(),
            project_id,
            root_project_id: None,
            target_revision: None,
            runtime_config_snapshot_ref: None,
            lease: WorkerLease {
                worker_id: PROJECT_MUTATION_WORKER_ID.into(),
                epoch: 1,
                fence_token: CONTROL_LEASE_FENCE.into(),
                expires_at: CONTROL_LEASE_EXPIRY.into(),
            },
            payload,
        };
        let response: WorkerResponse<O> = self
            .transport
            .command(operation, request)
            .await
            .map_err(worker_client_error)?;
        match response.status {
            WorkerOperationStatus::Completed => response.payload.ok_or_else(|| {
                AppError::ConfigError(format!(
                    "project mutation worker 完成响应缺少 payload: {operation}"
                ))
            }),
            WorkerOperationStatus::Failed => Err(worker_response_error(
                response.error.as_ref().expect("validated failed response"),
                operation,
            )),
            status => Err(AppError::ConfigError(format!(
                "project mutation worker 返回未完成状态: operation={operation}, status={status:?}"
            ))),
        }
    }

    fn positive_id(field: &str, value: i64) -> Result<i64, AppError> {
        if value <= 0 {
            return Err(AppError::InvalidArgument(format!("{field} 必须大于 0")));
        }
        Ok(value)
    }
}

#[async_trait::async_trait]
impl ProjectMutationApplicationPort for ProjectMutationWorkerClient {
    async fn create(
        &self,
        user_id: i64,
        input: CreateProjectInput,
    ) -> Result<ProjectSummary, AppError> {
        self.command(
            user_id,
            None,
            PROJECT_MUTATION_CREATE_OPERATION,
            WorkerCreateProjectInput {
                title: input.title,
                description: input.description,
                project_type: input.project_type,
            },
        )
        .await
    }

    async fn create_derived(
        &self,
        user_id: i64,
        input: CreateDerivedProjectInput,
    ) -> Result<ProjectSummary, AppError> {
        let root_project_id = Self::positive_id("root_project_id", input.root_project_id)?;
        self.command(
            user_id,
            Some(root_project_id),
            PROJECT_MUTATION_CREATE_DERIVED_OPERATION,
            WorkerCreateDerivedProjectInput {
                root_project_id,
                derived_kind: input.derived_kind,
                title: input.title,
                description: input.description,
                start_chapter: input.start_chapter,
                end_chapter: input.end_chapter,
            },
        )
        .await
    }

    async fn create_freeform_video(
        &self,
        user_id: i64,
        input: CreateFreeformVideoProjectInput,
    ) -> Result<ProjectSummary, AppError> {
        self.command(
            user_id,
            None,
            PROJECT_MUTATION_CREATE_FREEFORM_VIDEO_OPERATION,
            WorkerCreateFreeformVideoProjectInput {
                title: input.title,
                description: input.description,
            },
        )
        .await
    }

    async fn update(
        &self,
        user_id: i64,
        project_id: i64,
        input: UpdateProjectInput,
    ) -> Result<ProjectSummary, AppError> {
        let project_id = Self::positive_id("project_id", project_id)?;
        self.command(
            user_id,
            Some(project_id),
            PROJECT_MUTATION_UPDATE_OPERATION,
            WorkerUpdateProjectInput {
                project_id,
                title: input.title,
                description: input.description,
                fact_feedback_interval_chapters: input.fact_feedback_interval_chapters,
            },
        )
        .await
    }
}

fn worker_client_error(error: WorkerClientError) -> AppError {
    AppError::ConfigError(format!("project mutation worker 请求失败: {error}"))
}

fn worker_response_error(error: &WorkerError, operation: &str) -> AppError {
    match error.code {
        WorkerErrorCode::InvalidRequest => AppError::InvalidArgument(error.message.clone()),
        WorkerErrorCode::Unauthorized => AppError::Forbidden(error.message.clone()),
        WorkerErrorCode::NotFound => AppError::NotFound(error.message.clone()),
        WorkerErrorCode::Conflict => AppError::InvalidState {
            current: error.message.clone(),
            operation: operation.into(),
        },
        _ => AppError::ConfigError(format!(
            "project mutation worker {operation} 失败: {}",
            error.message
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn client_debug_redacts_worker_address_and_token() {
        let client = ProjectMutationWorkerClient::new(
            "https://project-mutation-worker.example.test",
            Some("project-mutation-token".into()),
        );
        let debug = format!("{client:?}");
        assert!(!debug.contains("project-mutation-worker.example.test"));
        assert!(!debug.contains("project-mutation-token"));
    }
}
