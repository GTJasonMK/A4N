//! Typed Host adapter for the chapter-scene worker.

use a4n_api_contract::chapter::{
    ChapterSceneMutationApplicationPort, ChapterSceneReadApplicationPort, ChapterSceneSummary,
    ReorderChapterScenesInput, UpdateChapterSceneInput,
};
use a4n_chapter_scene_worker_contract::{
    CHAPTER_SCENE_DELETE_OPERATION, CHAPTER_SCENE_LIST_OPERATION, CHAPTER_SCENE_REORDER_OPERATION,
    CHAPTER_SCENE_UPDATE_OPERATION, CHAPTER_SCENE_WORKER_ID, ChapterInput, DeleteSceneInput,
    ReorderScenesInput, UpdateSceneInput,
};
use a4n_error::AppError;
use a4n_worker_client::{HttpWorkerClient, WorkerClientError};
use a4n_worker_contract::{
    WORKER_SCHEMA_VERSION, WorkerEnvelope, WorkerError, WorkerErrorCode, WorkerLease,
    WorkerOperationStatus, WorkerPrincipal, WorkerPrincipalKind, WorkerResponse,
};

const DEFAULT_WORKER_URL: &str = "http://127.0.0.1:47670";
const CONTROL_LEASE_FENCE: &str = "control-request";
const CONTROL_LEASE_EXPIRY: &str = "9999-12-31T23:59:59Z";

#[derive(Clone)]
pub struct ChapterSceneWorkerClient {
    transport: HttpWorkerClient,
}

impl std::fmt::Debug for ChapterSceneWorkerClient {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ChapterSceneWorkerClient")
            .field("transport", &self.transport)
            .finish()
    }
}

impl ChapterSceneWorkerClient {
    pub fn new(base_url: impl Into<String>, auth_token: Option<String>) -> Self {
        let transport = HttpWorkerClient::new(base_url, CHAPTER_SCENE_WORKER_ID);
        let transport = match auth_token {
            Some(token) => transport.with_auth_token(token),
            None => transport,
        };
        Self { transport }
    }

    pub fn from_env() -> Self {
        let url = std::env::var("A4N_CHAPTER_SCENE_WORKER_URL")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| DEFAULT_WORKER_URL.to_string());
        let token = std::env::var("A4N_CHAPTER_SCENE_WORKER_TOKEN")
            .ok()
            .filter(|value| !value.trim().is_empty());
        Self::new(url, token)
    }

    async fn command<I, O>(&self, user_id: i64, operation: &str, payload: I) -> Result<O, AppError>
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
            project_id: None,
            root_project_id: None,
            target_revision: None,
            runtime_config_snapshot_ref: None,
            lease: WorkerLease {
                worker_id: CHAPTER_SCENE_WORKER_ID.into(),
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
                    "chapter scene worker 完成响应缺少 payload: {operation}"
                ))
            }),
            WorkerOperationStatus::Failed => Err(worker_response_error(
                response.error.as_ref().expect("validated failed response"),
                operation,
            )),
            status => Err(AppError::ConfigError(format!(
                "chapter scene worker 返回未完成状态: operation={operation}, status={status:?}"
            ))),
        }
    }

    fn positive(field: &str, value: i64) -> Result<i64, AppError> {
        if value <= 0 {
            return Err(AppError::InvalidArgument(format!("{field} 必须大于 0")));
        }
        Ok(value)
    }
}

#[async_trait::async_trait]
impl ChapterSceneReadApplicationPort for ChapterSceneWorkerClient {
    async fn list(
        &self,
        user_id: i64,
        chapter_id: i64,
    ) -> Result<Vec<ChapterSceneSummary>, AppError> {
        self.command(
            user_id,
            CHAPTER_SCENE_LIST_OPERATION,
            ChapterInput {
                chapter_id: Self::positive("chapter_id", chapter_id)?,
            },
        )
        .await
    }
}

#[async_trait::async_trait]
impl ChapterSceneMutationApplicationPort for ChapterSceneWorkerClient {
    async fn update(
        &self,
        user_id: i64,
        input: UpdateChapterSceneInput,
    ) -> Result<ChapterSceneSummary, AppError> {
        self.command(
            user_id,
            CHAPTER_SCENE_UPDATE_OPERATION,
            UpdateSceneInput {
                scene_id: Self::positive("scene_id", input.scene_id)?,
                content: input.content,
                title: input.title,
                summary: input.summary,
                scene_type: input.scene_type,
                location: input.location,
                timeline_position: input.timeline_position,
            },
        )
        .await
    }

    async fn delete(&self, user_id: i64, scene_id: i64) -> Result<(), AppError> {
        let _: serde_json::Value = self
            .command(
                user_id,
                CHAPTER_SCENE_DELETE_OPERATION,
                DeleteSceneInput {
                    scene_id: Self::positive("scene_id", scene_id)?,
                },
            )
            .await?;
        Ok(())
    }

    async fn reorder(
        &self,
        user_id: i64,
        input: ReorderChapterScenesInput,
    ) -> Result<Vec<ChapterSceneSummary>, AppError> {
        if input.order.is_empty() {
            return Err(AppError::InvalidArgument("order 不能为空".into()));
        }
        self.command(
            user_id,
            CHAPTER_SCENE_REORDER_OPERATION,
            ReorderScenesInput { order: input.order },
        )
        .await
    }
}

fn worker_client_error(error: WorkerClientError) -> AppError {
    AppError::ConfigError(format!("chapter scene worker 请求失败: {error}"))
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
            "chapter scene worker {operation} 失败: {}",
            error.message
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn client_debug_redacts_worker_address_and_token() {
        let client = ChapterSceneWorkerClient::new(
            "https://chapter-scene-worker.example.test",
            Some("chapter-scene-token".into()),
        );
        let debug = format!("{client:?}");
        assert!(!debug.contains("chapter-scene-worker.example.test"));
        assert!(!debug.contains("chapter-scene-token"));
    }
}
