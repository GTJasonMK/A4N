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
use a4n_worker_client::{
    HttpWorkerClient, WorkerClientErrorPolicy, map_worker_client_error, map_worker_response_error,
};
use a4n_worker_contract::{
    WORKER_SCHEMA_VERSION, WorkerEnvelope, WorkerError, WorkerLease, WorkerOperationStatus,
    WorkerPrincipal, WorkerPrincipalKind, WorkerResponse,
};

const DEFAULT_WORKER_URL: &str = "http://127.0.0.1:47670";

mod mutations;
mod reads;

#[derive(Clone, Debug)]
pub struct ChapterSceneWorkerClient {
    transport: HttpWorkerClient,
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
        let (url, token) = a4n_worker_client::worker_client_connection_from_env(
            "A4N_CHAPTER_SCENE_WORKER_URL",
            "A4N_CHAPTER_SCENE_WORKER_TOKEN",
            DEFAULT_WORKER_URL,
        );
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
            lease: WorkerLease::control_request(CHAPTER_SCENE_WORKER_ID),
            payload,
        };
        let response: WorkerResponse<O> = self
            .transport
            .command(operation, request)
            .await
            .map_err(|error| {
                map_worker_client_error(
                    error,
                    "chapter scene worker",
                    WorkerClientErrorPolicy::Configuration,
                )
            })?;
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

fn worker_response_error(error: &WorkerError, operation: &str) -> AppError {
    map_worker_response_error(error, "chapter scene worker", operation)
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
