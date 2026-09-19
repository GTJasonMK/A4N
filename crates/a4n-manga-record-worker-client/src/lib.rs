//! Typed Host adapter for the Manga record read worker.

use a4n_api_contract::manga::{
    MangaPagePromptSummary, MangaPipelineCancellationApplicationPort,
    MangaPipelineReadApplicationPort, MangaPipelineStatusSummary, MangaRecordReadApplicationPort,
};
use a4n_error::AppError;
use a4n_manga_record_worker_contract::{
    MANGA_PIPELINE_CANCEL_OPERATION, MANGA_PIPELINE_STATUS_OPERATION,
    MANGA_PIPELINE_STATUSES_OPERATION, MANGA_RECORD_LIST_OPERATION, MANGA_RECORD_WORKER_ID,
    MangaChapterInput, MangaPipelineStatusesInput,
};
use a4n_worker_client::{HttpWorkerClient, WorkerClientError};
use a4n_worker_contract::{
    WORKER_SCHEMA_VERSION, WorkerEnvelope, WorkerError, WorkerErrorCode, WorkerLease,
    WorkerOperationStatus, WorkerPrincipal, WorkerPrincipalKind, WorkerResponse,
};
use serde::Serialize;
use serde::de::DeserializeOwned;

const DEFAULT_WORKER_URL: &str = "http://127.0.0.1:47676";
const CONTROL_LEASE_FENCE: &str = "control-request";
const CONTROL_LEASE_EXPIRY: &str = "9999-12-31T23:59:59Z";

#[derive(Clone)]
pub struct MangaRecordWorkerClient {
    transport: HttpWorkerClient,
}

impl std::fmt::Debug for MangaRecordWorkerClient {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("MangaRecordWorkerClient")
            .field("transport", &self.transport)
            .finish()
    }
}

impl MangaRecordWorkerClient {
    pub fn new(base_url: impl Into<String>, auth_token: Option<String>) -> Self {
        let transport = HttpWorkerClient::new(base_url, MANGA_RECORD_WORKER_ID);
        let transport = match auth_token {
            Some(token) => transport.with_auth_token(token),
            None => transport,
        };
        Self { transport }
    }

    pub fn from_env() -> Self {
        let url = std::env::var("A4N_MANGA_RECORD_WORKER_URL")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| DEFAULT_WORKER_URL.to_string());
        let token = std::env::var("A4N_MANGA_RECORD_WORKER_TOKEN")
            .ok()
            .filter(|value| !value.trim().is_empty());
        Self::new(url, token)
    }

    async fn command<I, O>(
        &self,
        user_id: i64,
        project_id: i64,
        operation: &str,
        payload: I,
    ) -> Result<O, AppError>
    where
        I: Serialize,
        O: DeserializeOwned,
    {
        if user_id <= 0 {
            return Err(AppError::InvalidArgument("user_id 必须大于 0".into()));
        }
        if project_id <= 0 {
            return Err(AppError::InvalidArgument("project_id 必须大于 0".into()));
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
            project_id: Some(project_id),
            root_project_id: None,
            target_revision: None,
            runtime_config_snapshot_ref: None,
            lease: WorkerLease {
                worker_id: MANGA_RECORD_WORKER_ID.into(),
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
                    "manga record worker 完成响应缺少 payload: {operation}"
                ))
            }),
            WorkerOperationStatus::Failed => Err(worker_response_error(
                response.error.as_ref().expect("validated failed response"),
                operation,
            )),
            status => Err(AppError::ConfigError(format!(
                "manga record worker 返回未完成状态: operation={operation}, status={status:?}"
            ))),
        }
    }
}

#[async_trait::async_trait]
impl MangaRecordReadApplicationPort for MangaRecordWorkerClient {
    async fn list_page_prompts(
        &self,
        user_id: i64,
        project_id: i64,
        chapter_id: i64,
    ) -> Result<Vec<MangaPagePromptSummary>, AppError> {
        self.command(
            user_id,
            project_id,
            MANGA_RECORD_LIST_OPERATION,
            MangaChapterInput {
                project_id,
                chapter_id,
            },
        )
        .await
    }
}

#[async_trait::async_trait]
impl MangaPipelineReadApplicationPort for MangaRecordWorkerClient {
    async fn status(
        &self,
        user_id: i64,
        project_id: i64,
        chapter_id: i64,
    ) -> Result<MangaPipelineStatusSummary, AppError> {
        self.command(
            user_id,
            project_id,
            MANGA_PIPELINE_STATUS_OPERATION,
            MangaChapterInput {
                project_id,
                chapter_id,
            },
        )
        .await
    }

    async fn statuses(
        &self,
        user_id: i64,
        project_id: i64,
        chapter_ids: Vec<i64>,
    ) -> Result<Vec<MangaPipelineStatusSummary>, AppError> {
        self.command(
            user_id,
            project_id,
            MANGA_PIPELINE_STATUSES_OPERATION,
            MangaPipelineStatusesInput {
                project_id,
                chapter_ids,
            },
        )
        .await
    }
}

#[async_trait::async_trait]
impl MangaPipelineCancellationApplicationPort for MangaRecordWorkerClient {
    async fn cancel(
        &self,
        user_id: i64,
        project_id: i64,
        chapter_id: i64,
    ) -> Result<i64, AppError> {
        self.command(
            user_id,
            project_id,
            MANGA_PIPELINE_CANCEL_OPERATION,
            MangaChapterInput {
                project_id,
                chapter_id,
            },
        )
        .await
    }
}

fn worker_client_error(error: WorkerClientError) -> AppError {
    AppError::ConfigError(format!("manga record worker 请求失败: {error}"))
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
            "manga record worker {operation} 失败: {}",
            error.message
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn client_debug_redacts_worker_address_and_token() {
        let client = MangaRecordWorkerClient::new(
            "https://manga-record-worker.example.test",
            Some("manga-record-token".into()),
        );
        let debug = format!("{client:?}");
        assert!(!debug.contains("manga-record-worker.example.test"));
        assert!(!debug.contains("manga-record-token"));
    }
}
