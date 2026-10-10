//! Typed Host adapter for the video-record read worker.

use a4n_api_contract::image::GeneratedImageSummary;
use a4n_api_contract::video::{
    ChapterVideoPromptSummary, GeneratedVideoSummary, SelectSegmentVideoInput,
    UpdateGeneratedVideoInput, VideoPromptReadApplicationPort, VideoRecordReadApplicationPort,
};
use a4n_error::AppError;
use a4n_video_record_worker_contract::{
    ChapterVideosInput, LoadVideoSegmentPlanDraftInput, ReferenceImagesInput,
    UpdateVideoSegmentPlanDraftRecipeInput, VIDEO_PROMPT_DRAFT_LOAD_OPERATION,
    VIDEO_PROMPT_DRAFT_UPDATE_OPERATION, VIDEO_PROMPT_LIST_OPERATION,
    VIDEO_PROMPT_VALIDATE_SOURCE_OPERATION, VIDEO_RECORD_LIST_OPERATION,
    VIDEO_RECORD_PLAYBACK_OPERATION, VIDEO_RECORD_REFERENCE_IMAGES_OPERATION,
    VIDEO_RECORD_SELECT_SEGMENT_OPERATION, VIDEO_RECORD_UPDATE_OPERATION, VIDEO_RECORD_WORKER_ID,
    VideoIdInput, VideoSegmentSelectionInput, VideoSourceInput, VideoUpdateInput,
};
#[cfg(test)]
use a4n_worker_client::WorkerClientError;
use a4n_worker_client::{HttpWorkerClient, WorkerClientErrorPolicy, map_worker_client_error};
use a4n_worker_contract::{
    WORKER_SCHEMA_VERSION, WorkerEnvelope, WorkerError, WorkerErrorCode, WorkerLease,
    WorkerOperationStatus, WorkerPrincipal, WorkerPrincipalKind, WorkerResponse,
};
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;

const DEFAULT_WORKER_URL: &str = "http://127.0.0.1:47675";

mod mutations;
mod prompts;
mod reads;

#[derive(Clone, Debug)]
pub struct VideoRecordWorkerClient {
    transport: HttpWorkerClient,
}

impl VideoRecordWorkerClient {
    pub fn new(base_url: impl Into<String>, auth_token: Option<String>) -> Self {
        let transport = HttpWorkerClient::new(base_url, VIDEO_RECORD_WORKER_ID);
        let transport = match auth_token {
            Some(token) => transport.with_auth_token(token),
            None => transport,
        };
        Self { transport }
    }

    pub fn from_env() -> Self {
        let (url, token) = a4n_worker_client::worker_client_connection_from_env(
            "A4N_VIDEO_RECORD_WORKER_URL",
            "A4N_VIDEO_RECORD_WORKER_TOKEN",
            DEFAULT_WORKER_URL,
        );
        Self::new(url, token)
    }

    async fn command_with_principal<I, O>(
        &self,
        principal: WorkerPrincipal,
        project_id: Option<i64>,
        operation: &str,
        payload: I,
    ) -> Result<O, AppError>
    where
        I: Serialize,
        O: DeserializeOwned,
    {
        let request_id = uuid::Uuid::new_v4().to_string();
        let request = WorkerEnvelope {
            schema_version: WORKER_SCHEMA_VERSION,
            request_id: request_id.clone(),
            trace_id: request_id,
            principal,
            operation_id: operation.into(),
            idempotency_key: uuid::Uuid::new_v4().to_string(),
            project_id,
            root_project_id: None,
            target_revision: None,
            runtime_config_snapshot_ref: None,
            lease: WorkerLease::control_request(VIDEO_RECORD_WORKER_ID),
            payload,
        };
        let response: WorkerResponse<O> = self
            .transport
            .command(operation, request)
            .await
            .map_err(|error| {
                map_worker_client_error(
                    error,
                    "video record worker",
                    WorkerClientErrorPolicy::TransientAsTimeout,
                )
            })?;
        match response.status {
            WorkerOperationStatus::Completed => response.payload.ok_or_else(|| {
                AppError::ConfigError(format!(
                    "video record worker 完成响应缺少 payload: {operation}"
                ))
            }),
            WorkerOperationStatus::Failed => Err(worker_response_error(
                response.error.as_ref().expect("validated failed response"),
                operation,
            )),
            status => Err(AppError::ConfigError(format!(
                "video record worker 返回未完成状态: operation={operation}, status={status:?}"
            ))),
        }
    }

    async fn command<I, O>(
        &self,
        user_id: i64,
        project_id: Option<i64>,
        operation: &str,
        payload: I,
    ) -> Result<O, AppError>
    where
        I: Serialize,
        O: DeserializeOwned,
    {
        let principal = a4n_worker_client::user_principal(user_id)?;
        self.command_with_principal(principal, project_id, operation, payload)
            .await
    }

    fn service_principal() -> WorkerPrincipal {
        WorkerPrincipal {
            kind: WorkerPrincipalKind::Service,
            subject: "runtime".into(),
        }
    }
}

fn worker_response_error(error: &WorkerError, operation: &str) -> AppError {
    match error.code {
        WorkerErrorCode::InvalidRequest => AppError::InvalidArgument(error.message.clone()),
        WorkerErrorCode::Unauthorized => AppError::Forbidden(error.message.clone()),
        WorkerErrorCode::NotFound => AppError::NotFound(error.message.clone()),
        WorkerErrorCode::Conflict | WorkerErrorCode::VersionMismatch => AppError::InvalidState {
            current: error.message.clone(),
            operation: operation.into(),
        },
        WorkerErrorCode::Unavailable | WorkerErrorCode::Timeout if error.retryable => {
            AppError::Timeout(format!(
                "video record worker {operation} 失败: {}",
                error.message
            ))
        }
        WorkerErrorCode::Cancelled => AppError::Cancelled,
        _ => AppError::ConfigError(format!(
            "video record worker {operation} 失败: {}",
            error.message
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn client_debug_redacts_worker_address_and_token() {
        let client = VideoRecordWorkerClient::new(
            "https://video-record-worker.example.test",
            Some("video-record-token".into()),
        );
        let debug = format!("{client:?}");
        assert!(!debug.contains("video-record-worker.example.test"));
        assert!(!debug.contains("video-record-token"));
    }

    #[test]
    fn retryable_worker_failures_remain_retryable() {
        assert!(matches!(
            map_worker_client_error(
                WorkerClientError::Timeout,
                "video record worker",
                WorkerClientErrorPolicy::TransientAsTimeout,
            ),
            AppError::Timeout(_)
        ));
        assert!(matches!(
            map_worker_client_error(
                WorkerClientError::HttpStatus { status: 503 },
                "video record worker",
                WorkerClientErrorPolicy::TransientAsTimeout,
            ),
            AppError::Timeout(_)
        ));
        assert!(matches!(
            worker_response_error(
                &WorkerError {
                    code: WorkerErrorCode::Unavailable,
                    message: "database unavailable".into(),
                    retryable: true,
                },
                VIDEO_RECORD_LIST_OPERATION,
            ),
            AppError::Timeout(_)
        ));
    }
}
