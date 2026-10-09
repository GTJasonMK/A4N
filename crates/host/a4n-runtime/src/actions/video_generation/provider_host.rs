mod canvas;
mod chapter;
mod reference_media;
mod video_provider;

use a4n_domain::cost::{
    ModelCapability, ModelUsageMeter, ModelUsageRecordCompletion, ModelUsageRecordStart,
};
use a4n_error::AppError;
use a4n_provider_contract::VideoProviderConfig;
use a4n_queue::priority::{TaskKind as QueueTaskKind, TaskPriority};
use a4n_video::{ProviderImageInputTransport, VideoGenerationResult};
use a4n_video_host_contract::{
    ChapterVideoImageInput, ResolvedChapterVideoImageInputs, VideoGenerationProgressPort,
    VideoProviderRequestBody, VideoProviderRequestInput,
};
use serde_json::{Value, json};
use tokio_util::sync::CancellationToken;

use crate::{
    AppState,
    actions::video_generation::{
        common::relay_video_sidecar_progress, video_sidecar_root_project_id,
    },
};
use a4n_video_sidecar_contract::VideoGenerationInput;

pub(crate) struct RuntimeVideoProviderHostPort<'state> {
    pub(crate) state: &'state AppState,
}

impl<'state> RuntimeVideoProviderHostPort<'state> {
    pub(crate) fn new(state: &'state AppState) -> Self {
        Self { state }
    }

    async fn resolve_config(
        &self,
        config_id: Option<i64>,
    ) -> Result<VideoProviderConfig, AppError> {
        super::config::resolve_provider_config(self.state, config_id).await
    }

    fn resolve_image_transport_setting(
        &self,
        backend: &str,
    ) -> Result<ProviderImageInputTransport, AppError> {
        super::config::resolve_image_transport(backend)
    }

    async fn provider_request_body(
        &self,
        mut input: VideoProviderRequestInput,
    ) -> Result<VideoProviderRequestBody, AppError> {
        self.assemble_video_prompt(input.project_id, &mut input.request)
            .await?;
        super::preview::provider_request_body(self.state, input).await
    }

    async fn assemble_video_prompt(
        &self,
        project_id: i64,
        request: &mut VideoGenerationInput,
    ) -> Result<(), AppError> {
        request.prompt = a4n_api_contract::prompt::PromptRenderPort::render_prompt_for_call(
            self.state,
            "video_generation.provider",
            "video_generation_request",
            &std::collections::HashMap::from([("prompt".to_string(), json!(request.prompt))]),
            Some(project_id),
        )
        .await?;
        Ok(())
    }

    async fn submit_video(
        &self,
        mut input: VideoProviderRequestInput,
        progress: &mut dyn VideoGenerationProgressPort,
    ) -> Result<VideoGenerationResult, AppError> {
        let root_project_id = video_sidecar_root_project_id(
            self.state.runtime_profile(),
            self.state.project_access_application(),
            input.project_id,
        )
        .await
        .map_err(host_error)?;
        let cost_config = input.config.clone();
        let duration_seconds = input.request.duration_secs.to_string();
        let request_json = match self.provider_request_body(input.clone()).await {
            Ok(body) => Some(body.request),
            Err(error) => {
                tracing::warn!(backend = %cost_config.backend, %error, "Could not capture video request snapshot");
                None
            }
        };
        self.assemble_video_prompt(input.project_id, &mut input.request)
            .await?;
        let cost_record = self
            .state
            .provider_runtime()
            .start_model_usage(ModelUsageRecordStart {
                request_id: uuid::Uuid::new_v4().to_string(),
                user_id: Some(input.user_id),
                project_id: Some(input.project_id),
                root_project_id,
                workflow_run_id: None,
                generation_run_id: None,
                workflow: "video_generation".to_string(),
                step_key: "provider_submit".to_string(),
                call_key: None,
                source: Some("video_generation".to_string()),
                capability: ModelCapability::Video,
                provider_config_id: Some(cost_config.id),
                provider: Some(cost_config.backend.clone()),
                model: Some(cost_config.model.clone()),
                pricing: cost_config.pricing.clone(),
                request_json,
            })
            .await
            .map_err(|error| host_error(error.to_string()))?;
        let (sidecar_progress_tx, sidecar_progress_rx) = tokio::sync::mpsc::unbounded_channel();
        let provider_cancel = CancellationToken::new();
        let cancellation = progress.cancellation_handle();
        let request_future = async move {
            let request = self.state.provider_runtime().queue_run_tracked_with_cancel(
                QueueTaskKind::VideoGeneration,
                TaskPriority::Medium,
                Some(input.project_id),
                provider_cancel.clone(),
                {
                    let state = self.state;
                    move || async move {
                        crate::sidecar_clients::generate_video(
                            state.runtime_profile(),
                            state.capabilities().video.clone(),
                            state.data_dir(),
                            &input.config,
                            input.request,
                            input.provider_idempotency_key_sha256,
                            Some(input.project_id),
                            root_project_id,
                            None,
                            Some(sidecar_progress_tx),
                        )
                        .await
                    }
                },
            );
            tokio::pin!(request);
            tokio::select! {
                result = &mut request => result,
                _ = cancellation.cancelled() => {
                    provider_cancel.cancel();
                    (&mut request).await
                }
            }
        };
        let result =
            relay_video_sidecar_progress(progress, sidecar_progress_rx, request_future).await;
        let status = if result.is_ok() {
            "completed"
        } else {
            "failed"
        };
        self.state
            .provider_runtime()
            .finish_model_usage(
                cost_record,
                ModelUsageRecordCompletion {
                    status: status.to_string(),
                    meter: ModelUsageMeter {
                        duration_seconds: Some(duration_seconds),
                        ..Default::default()
                    },
                    error_kind: result.as_ref().err().map(|_| "provider".to_string()),
                    error_message: result.as_ref().err().map(ToString::to_string),
                },
            )
            .await
            .map_err(|error| host_error(error.to_string()))?;
        result
    }

    async fn resolve_video_image_inputs(
        &self,
        input: ChapterVideoImageInput,
    ) -> Result<ResolvedChapterVideoImageInputs, AppError> {
        super::image_inputs::resolve_chapter_video_image_inputs(self.state, input).await
    }

    fn redacted_runtime_config_snapshot(&self) -> Option<Value> {
        crate::runtime_config::redacted_current_snapshot_value()
    }
}

pub(super) fn host_error(message: String) -> AppError {
    AppError::ConfigError(message)
}
