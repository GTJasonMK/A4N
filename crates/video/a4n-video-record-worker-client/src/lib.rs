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
use a4n_worker_client::{HttpWorkerClient, WorkerClientError};
use a4n_worker_contract::{
    WORKER_SCHEMA_VERSION, WorkerEnvelope, WorkerError, WorkerErrorCode, WorkerLease,
    WorkerOperationStatus, WorkerPrincipal, WorkerPrincipalKind, WorkerResponse,
};
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;

const DEFAULT_WORKER_URL: &str = "http://127.0.0.1:47675";
const CONTROL_LEASE_FENCE: &str = "control-request";
const CONTROL_LEASE_EXPIRY: &str = "9999-12-31T23:59:59Z";

#[derive(Clone)]
pub struct VideoRecordWorkerClient {
    transport: HttpWorkerClient,
}

impl std::fmt::Debug for VideoRecordWorkerClient {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("VideoRecordWorkerClient")
            .field("transport", &self.transport)
            .finish()
    }
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
        let url = std::env::var("A4N_VIDEO_RECORD_WORKER_URL")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| DEFAULT_WORKER_URL.to_string());
        let token = std::env::var("A4N_VIDEO_RECORD_WORKER_TOKEN")
            .ok()
            .filter(|value| !value.trim().is_empty());
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
            lease: WorkerLease {
                worker_id: VIDEO_RECORD_WORKER_ID.into(),
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
        if user_id <= 0 {
            return Err(AppError::InvalidArgument("user_id 必须大于 0".into()));
        }
        self.command_with_principal(
            WorkerPrincipal {
                kind: WorkerPrincipalKind::User,
                subject: user_id.to_string(),
            },
            project_id,
            operation,
            payload,
        )
        .await
    }

    fn service_principal() -> WorkerPrincipal {
        WorkerPrincipal {
            kind: WorkerPrincipalKind::Service,
            subject: "runtime".into(),
        }
    }

    pub async fn update_record(
        &self,
        user_id: i64,
        video_id: i64,
        input: UpdateGeneratedVideoInput,
    ) -> Result<GeneratedVideoSummary, AppError> {
        self.command(
            user_id,
            None,
            VIDEO_RECORD_UPDATE_OPERATION,
            VideoUpdateInput { video_id, input },
        )
        .await
    }

    pub async fn select_segment(
        &self,
        user_id: i64,
        project_id: i64,
        chapter_id: i64,
        video_id: i64,
        input: SelectSegmentVideoInput,
    ) -> Result<GeneratedVideoSummary, AppError> {
        self.command(
            user_id,
            Some(project_id),
            VIDEO_RECORD_SELECT_SEGMENT_OPERATION,
            VideoSegmentSelectionInput {
                project_id,
                chapter_id,
                video_id,
                input,
            },
        )
        .await
    }

    pub async fn load_video_segment_plan_draft(
        &self,
        user_id: i64,
        input: LoadVideoSegmentPlanDraftInput,
    ) -> Result<a4n_api_contract::video::VideoSegmentPlanDraftSummary, AppError> {
        self.command(
            user_id,
            Some(input.project_id),
            VIDEO_PROMPT_DRAFT_LOAD_OPERATION,
            input,
        )
        .await
    }

    pub async fn update_video_segment_plan_draft_recipe(
        &self,
        user_id: i64,
        input: UpdateVideoSegmentPlanDraftRecipeInput,
    ) -> Result<(), AppError> {
        self.command(
            user_id,
            Some(input.project_id),
            VIDEO_PROMPT_DRAFT_UPDATE_OPERATION,
            input,
        )
        .await
    }
}

#[async_trait::async_trait]
impl VideoRecordReadApplicationPort for VideoRecordWorkerClient {
    async fn playback_source(
        &self,
        user_id: i64,
        video_id: i64,
    ) -> Result<GeneratedVideoSummary, AppError> {
        self.command(
            user_id,
            None,
            VIDEO_RECORD_PLAYBACK_OPERATION,
            VideoIdInput { video_id },
        )
        .await
    }

    async fn list_by_chapter(
        &self,
        user_id: i64,
        project_id: i64,
        chapter_id: i64,
    ) -> Result<Vec<GeneratedVideoSummary>, AppError> {
        self.command(
            user_id,
            Some(project_id),
            VIDEO_RECORD_LIST_OPERATION,
            ChapterVideosInput {
                project_id,
                chapter_id,
            },
        )
        .await
    }

    async fn validated_reference_images(
        &self,
        user_id: i64,
        project_id: i64,
        chapter_id: i64,
        reference_image_ids_json: Option<&Value>,
    ) -> Result<Vec<GeneratedImageSummary>, AppError> {
        self.command(
            user_id,
            Some(project_id),
            VIDEO_RECORD_REFERENCE_IMAGES_OPERATION,
            ReferenceImagesInput {
                project_id,
                chapter_id,
                reference_image_ids_json: reference_image_ids_json.cloned(),
            },
        )
        .await
    }
}

#[async_trait::async_trait]
impl VideoPromptReadApplicationPort for VideoRecordWorkerClient {
    async fn list_chapter(
        &self,
        user_id: i64,
        project_id: i64,
        chapter_id: i64,
    ) -> Result<Vec<ChapterVideoPromptSummary>, AppError> {
        self.command(
            user_id,
            Some(project_id),
            VIDEO_PROMPT_LIST_OPERATION,
            ChapterVideosInput {
                project_id,
                chapter_id,
            },
        )
        .await
    }

    async fn validate_project_chapter_source(
        &self,
        project_id: i64,
        chapter_id: i64,
        operation: &str,
        writable: bool,
    ) -> Result<(), AppError> {
        self.command_with_principal(
            Self::service_principal(),
            Some(project_id),
            VIDEO_PROMPT_VALIDATE_SOURCE_OPERATION,
            VideoSourceInput {
                project_id,
                chapter_id,
                operation: operation.to_string(),
                writable,
            },
        )
        .await
    }
}

fn worker_client_error(error: WorkerClientError) -> AppError {
    AppError::ConfigError(format!("video record worker 请求失败: {error}"))
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
}
