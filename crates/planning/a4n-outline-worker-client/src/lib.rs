//! Typed Host adapter for the outline records worker.

use a4n_api_contract::outline::{
    ChapterOutlineSummary, DeleteLatestOutlineRecordsInput, OutlineGenerationProgressSummary,
    OutlineMutationApplicationPort, OutlineMutationResult, OutlineReadApplicationPort,
    PartOutlineSummary, SaveChapterOutlineInput, SaveChapterOutlineResult, SavePartOutlineInput,
    SavePartOutlineResult, UpdateChapterOutlineInput, UpdateChapterOutlineResult,
};
use a4n_error::AppError;
use a4n_outline_worker_contract::{
    ChapterOutlineAccessInput, DELETE_CHAPTERS_OPERATION, DELETE_PARTS_OPERATION,
    ENSURE_CHAPTER_ACCESS_OPERATION, ENSURE_PART_ACCESS_OPERATION, GENERATION_PROGRESS_OPERATION,
    LIST_CHAPTERS_OPERATION, LIST_PARTS_OPERATION, OUTLINE_WORKER_ID, PartOutlineAccessInput,
    ProjectOutlineInput, SAVE_CHAPTER_OPERATION, SAVE_PART_OPERATION, UPDATE_CHAPTER_OPERATION,
};
use a4n_worker_client::{HttpWorkerClient, WorkerClientError};
use a4n_worker_contract::{
    WORKER_SCHEMA_VERSION, WorkerEnvelope, WorkerError, WorkerErrorCode, WorkerLease,
    WorkerOperationStatus, WorkerPrincipal, WorkerPrincipalKind, WorkerResponse,
};

const DEFAULT_WORKER_URL: &str = "http://127.0.0.1:47653";
const CONTROL_LEASE_FENCE: &str = "control-request";
const CONTROL_LEASE_EXPIRY: &str = "9999-12-31T23:59:59Z";

#[derive(Clone)]
pub struct OutlineWorkerClient {
    transport: HttpWorkerClient,
}

impl std::fmt::Debug for OutlineWorkerClient {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("OutlineWorkerClient")
            .field("transport", &self.transport)
            .finish()
    }
}

impl OutlineWorkerClient {
    pub fn new(base_url: impl Into<String>, auth_token: Option<String>) -> Self {
        let transport = HttpWorkerClient::new(base_url, OUTLINE_WORKER_ID);
        let transport = match auth_token {
            Some(token) => transport.with_auth_token(token),
            None => transport,
        };
        Self { transport }
    }

    pub fn from_env() -> Self {
        let url = std::env::var("A4N_OUTLINE_WORKER_URL")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| DEFAULT_WORKER_URL.to_string());
        let token = std::env::var("A4N_OUTLINE_WORKER_TOKEN")
            .ok()
            .filter(|value| !value.trim().is_empty());
        Self::new(url, token)
    }

    async fn command<I, O>(
        &self,
        user_id: i64,
        project_id: i64,
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
            target_revision: None,
            runtime_config_snapshot_ref: None,
            lease: WorkerLease {
                worker_id: OUTLINE_WORKER_ID.into(),
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
                    "outline worker 完成响应缺少 payload: {operation_id}"
                ))
            }),
            WorkerOperationStatus::Failed => Err(worker_response_error(
                response.error.as_ref().expect("validated failed response"),
                operation_id,
            )),
            status => Err(AppError::ConfigError(format!(
                "outline worker 返回未完成状态: operation={operation_id}, status={status:?}"
            ))),
        }
    }
}

#[async_trait::async_trait]
impl OutlineReadApplicationPort for OutlineWorkerClient {
    async fn list_chapters(
        &self,
        user_id: i64,
        project_id: i64,
    ) -> Result<Vec<ChapterOutlineSummary>, AppError> {
        self.command(
            user_id,
            project_id,
            LIST_CHAPTERS_OPERATION,
            ProjectOutlineInput { project_id },
        )
        .await
    }

    async fn list_parts(
        &self,
        user_id: i64,
        project_id: i64,
    ) -> Result<Vec<PartOutlineSummary>, AppError> {
        self.command(
            user_id,
            project_id,
            LIST_PARTS_OPERATION,
            ProjectOutlineInput { project_id },
        )
        .await
    }

    async fn generation_progress(
        &self,
        user_id: i64,
        project_id: i64,
    ) -> Result<OutlineGenerationProgressSummary, AppError> {
        self.command(
            user_id,
            project_id,
            GENERATION_PROGRESS_OPERATION,
            ProjectOutlineInput { project_id },
        )
        .await
    }

    async fn ensure_chapter_outline_access(
        &self,
        user_id: i64,
        project_id: i64,
        outline_id: i64,
    ) -> Result<(), AppError> {
        self.command::<_, ()>(
            user_id,
            project_id,
            ENSURE_CHAPTER_ACCESS_OPERATION,
            ChapterOutlineAccessInput {
                project_id,
                outline_id,
            },
        )
        .await
    }

    async fn ensure_part_access(
        &self,
        user_id: i64,
        project_id: i64,
        part_id: i64,
    ) -> Result<(), AppError> {
        self.command::<_, ()>(
            user_id,
            project_id,
            ENSURE_PART_ACCESS_OPERATION,
            PartOutlineAccessInput {
                project_id,
                part_id,
            },
        )
        .await
    }
}

#[async_trait::async_trait]
impl OutlineMutationApplicationPort for OutlineWorkerClient {
    async fn delete_latest_chapters(
        &self,
        user_id: i64,
        input: DeleteLatestOutlineRecordsInput,
    ) -> Result<OutlineMutationResult, AppError> {
        self.command(user_id, input.project_id, DELETE_CHAPTERS_OPERATION, input)
            .await
    }

    async fn delete_latest_parts(
        &self,
        user_id: i64,
        input: DeleteLatestOutlineRecordsInput,
    ) -> Result<OutlineMutationResult, AppError> {
        self.command(user_id, input.project_id, DELETE_PARTS_OPERATION, input)
            .await
    }

    async fn save_chapter(
        &self,
        user_id: i64,
        input: SaveChapterOutlineInput,
    ) -> Result<SaveChapterOutlineResult, AppError> {
        self.command(user_id, input.project_id, SAVE_CHAPTER_OPERATION, input)
            .await
    }

    async fn update_chapter(
        &self,
        user_id: i64,
        input: UpdateChapterOutlineInput,
    ) -> Result<UpdateChapterOutlineResult, AppError> {
        self.command(user_id, input.project_id, UPDATE_CHAPTER_OPERATION, input)
            .await
    }

    async fn save_part(
        &self,
        user_id: i64,
        input: SavePartOutlineInput,
    ) -> Result<SavePartOutlineResult, AppError> {
        self.command(user_id, input.project_id, SAVE_PART_OPERATION, input)
            .await
    }
}

fn worker_client_error(error: WorkerClientError) -> AppError {
    AppError::ConfigError(format!("outline worker 请求失败: {error}"))
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
        _ => AppError::ConfigError(format!(
            "outline worker {operation} 失败: {}",
            error.message
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn client_debug_redacts_worker_address_and_token() {
        let client = OutlineWorkerClient::new(
            "https://outline-worker.example.test",
            Some("outline-token".into()),
        );
        let debug = format!("{client:?}");
        assert!(!debug.contains("outline-worker.example.test"));
        assert!(!debug.contains("outline-token"));
    }
}
