use a4n_canvas_host_contract::{
    CanvasVideoFileInput, CanvasVideoGenerationHostPort, CanvasVideoPersistedFile,
    CanvasVideoProviderInput,
};
use a4n_queue::TaskWorkflowContext;
use a4n_video::VideoJobState;
use a4n_video_host_contract::{
    CanvasVideoImageInput, CanvasVideoPreviewHostPort, CanvasVideoResolvedImages,
    VideoCancellationHandle, VideoProviderHostPort,
};
use async_trait::async_trait;
use tokio_util::sync::CancellationToken;

use crate::{
    actions::video_generation::media::persist_canvas_video_bytes,
    cleanup_outbox::enqueue_file_artifact_cleanup_by_project,
};

use super::*;

fn video_cancellation_handle(token: CancellationToken) -> VideoCancellationHandle {
    let cancelled = token;
    VideoCancellationHandle::new(move || {
        let token = cancelled.clone();
        Box::pin(async move { token.cancelled().await })
    })
}

#[async_trait]
impl CanvasVideoPreviewHostPort for RuntimeVideoProviderHostPort<'_> {
    async fn resolve_video_config_for_preview(
        &self,
        config_id: Option<i64>,
    ) -> Result<VideoProviderConfig, AppError> {
        self.resolve_config(config_id).await
    }

    async fn resolve_canvas_video_images_for_preview(
        &self,
        input: CanvasVideoImageInput,
    ) -> Result<CanvasVideoResolvedImages, AppError> {
        let transport = self.resolve_image_transport_setting(&input.backend)?;
        let resolved = self
            .resolve_video_image_inputs(ChapterVideoImageInput {
                user_id: input.user_id,
                project_id: input.project_id,
                chapter_id: None,
                mode: input.mode,
                transport,
                reference_image_ids: input.reference_image_ids,
                first_frame_image_id: input.first_frame_image_id,
                last_frame_image_id: input.last_frame_image_id,
            })
            .await?;
        Ok(CanvasVideoResolvedImages {
            first_frame_image_url: resolved.first_frame_image_url,
            last_frame_image_url: resolved.last_frame_image_url,
            reference_image_urls: resolved.reference_image_urls,
        })
    }
}

#[async_trait]
impl CanvasVideoGenerationHostPort for RuntimeVideoProviderHostPort<'_> {
    async fn resolve_video_config(
        &self,
        config_id: Option<i64>,
    ) -> Result<VideoProviderConfig, AppError> {
        self.resolve_config(config_id).await
    }

    async fn resolve_canvas_video_images(
        &self,
        input: CanvasVideoImageInput,
    ) -> Result<CanvasVideoResolvedImages, AppError> {
        <Self as CanvasVideoPreviewHostPort>::resolve_canvas_video_images_for_preview(self, input)
            .await
    }

    async fn submit_canvas_video(
        &self,
        input: CanvasVideoProviderInput,
    ) -> Result<VideoGenerationResult, AppError> {
        let root_project_id = video_sidecar_root_project_id(
            self.state.runtime_profile(),
            self.state.project_access_application(),
            input.project_id,
        )
        .await
        .map_err(host_error)?;
        let duration_seconds = input.request.duration_secs.to_string();
        let request_json = match self
            .provider_request_body(VideoProviderRequestInput {
                user_id: input.user_id,
                project_id: input.project_id,
                config: input.config.clone(),
                request: input.request.clone(),
                provider_idempotency_key_sha256: None,
            })
            .await
        {
            Ok(body) => Some(body.request),
            Err(error) => {
                tracing::warn!(backend = %input.config.backend, %error, "Could not capture canvas video request snapshot");
                None
            }
        };
        let cost_record = self
            .state
            .provider_runtime()
            .start_model_usage(ModelUsageRecordStart {
                request_id: uuid::Uuid::new_v4().to_string(),
                user_id: Some(input.user_id),
                project_id: Some(input.project_id),
                root_project_id,
                workflow_run_id: Some(input.workflow_run_id),
                generation_run_id: None,
                workflow: "canvas_video_generation".to_string(),
                step_key: "provider_submit".to_string(),
                call_key: None,
                source: Some("video_generation".to_string()),
                capability: ModelCapability::Video,
                provider_config_id: Some(input.config.id),
                provider: Some(input.config.backend.clone()),
                model: Some(input.config.model.clone()),
                pricing: input.config.pricing.clone(),
                request_json,
            })
            .await
            .map_err(|error| host_error(error.to_string()))?;
        let workflow = TaskWorkflowContext::new(input.workflow_run_id)?;
        let result = self
            .state
            .provider_runtime()
            .queue_run_tracked_with_workflow_context_and_cancel(
                QueueTaskKind::VideoGeneration,
                TaskPriority::Medium,
                Some(input.project_id),
                Some(workflow),
                CancellationToken::new(),
                || async move {
                    crate::sidecar_clients::generate_video(
                        self.state.runtime_profile(),
                        self.state.capabilities().video.clone(),
                        self.state.data_dir(),
                        &input.config,
                        input.request,
                        None,
                        Some(input.project_id),
                        root_project_id,
                        Some(input.workflow_run_id),
                        None,
                    )
                    .await
                },
            )
            .await;
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

    async fn poll_canvas_video(
        &self,
        project_id: i64,
        workflow_run_id: i64,
        config: &VideoProviderConfig,
        job_id: &str,
    ) -> Result<VideoJobState, AppError> {
        let cancellation = video_cancellation_handle(CancellationToken::new());
        <Self as VideoProviderHostPort>::poll_video_job(
            self,
            project_id,
            workflow_run_id,
            config,
            job_id,
            &cancellation,
        )
        .await
    }

    async fn persist_canvas_video_file(
        &self,
        input: CanvasVideoFileInput<'_>,
    ) -> Result<CanvasVideoPersistedFile, AppError> {
        let (relative_path, file_size) = persist_canvas_video_bytes(
            self.state.data_dir(),
            input.project_id,
            input.video_id,
            input.bytes,
            input.format,
        )
        .map_err(host_error)?;
        let path = self.state.data_dir().join(&relative_path);
        let properties = a4n_storage::remove_created_file_on_error(&path, "画布视频文件", async {
            self.state
                .probe_media_video_properties(
                    &path,
                    a4n_media_contract::MediaRoot::Data,
                    "画布视频",
                )
                .await
                .map(|probe| a4n_media::video_file_properties(&probe, file_size))
        })
        .await
        .map_err(host_error)?;
        Ok(CanvasVideoPersistedFile {
            relative_path,
            properties,
        })
    }

    async fn remove_canvas_video_file(&self, relative_path: &str) -> Result<(), AppError> {
        enqueue_file_artifact_cleanup_by_project(
            self.state,
            &format!("canvas-video-cleanup:{relative_path}"),
            &[relative_path.to_string()],
        )
        .await
        .map_err(host_error)
    }

    fn runtime_config_snapshot(&self) -> Option<Value> {
        self.redacted_runtime_config_snapshot()
    }
}
