use a4n_canvas_host_contract::{
    CanvasVideoFileInput, CanvasVideoGenerationHostPort, CanvasVideoPersistedFile,
    CanvasVideoProviderInput,
};
use a4n_domain::cost::{
    ModelCapability, ModelUsageMeter, ModelUsageRecordCompletion, ModelUsageRecordStart,
};
use a4n_domain::media::{GeneratedMediaArtifactFile, video_mime_from_path};
use a4n_error::AppError;
use a4n_provider_contract::VideoProviderConfig;
use a4n_queue::{
    TaskWorkflowContext,
    priority::{TaskKind as QueueTaskKind, TaskPriority},
};
use a4n_service_protocol::GenerationArtifactRef;
use a4n_video::{
    ProviderImageInputTransport, VideoGenerationResult, VideoJobState,
    is_canvas_video_record_fields,
};
use a4n_video_host_contract::{
    CanvasVideoImageInput, CanvasVideoPreviewHostPort, CanvasVideoResolvedImages,
    ChapterVideoGenerationHostPort, ChapterVideoImageInput, ChapterVideoPreviewHostPort,
    PersistedVideoResult, ReferenceMediaVideoGenerationHostPort,
    ReferenceMediaVideoPreviewHostPort, ResolvedChapterVideoImageInputs, VideoCancellationHandle,
    VideoGenerationProgressPort, VideoProviderHostPort, VideoProviderRequestBody,
    VideoProviderRequestInput, VideoProviderResultTarget, VideoProviderValidationMode,
};
use async_trait::async_trait;
use serde_json::{Value, json};
use tokio_util::sync::CancellationToken;

use crate::{
    AppState,
    actions::video_generation::{
        common::relay_video_sidecar_progress,
        job_lifecycle::poll_provider_job,
        media::{
            delete_video_tail_frame_assets, media_bytes_to_remote_temp_media_url,
            persist_canvas_video_bytes, reference_image_provider_input_by_id,
            reference_image_provider_inputs_by_ids, reference_image_request_preview_url_by_id,
            reference_image_request_preview_urls_by_ids,
        },
        video_sidecar_root_project_id,
    },
    cleanup_outbox::enqueue_file_artifact_cleanup_by_project,
    file_utils::inspect_generated_media_file,
};

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
        input: VideoProviderRequestInput,
    ) -> Result<VideoProviderRequestBody, AppError> {
        super::preview::provider_request_body(self.state, input).await
    }

    async fn submit_video(
        &self,
        input: VideoProviderRequestInput,
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
        let video = self.state.video_record_read_application();
        match input.mode {
            VideoProviderValidationMode::Generate => {
                let jwt_secret = if input.transport == ProviderImageInputTransport::PublicUrl {
                    Some(self.state.required_jwt_secret()?)
                } else {
                    None
                };
                if input.transport == ProviderImageInputTransport::RemoteTempUpload {
                    let mut staging_artifacts = Vec::new();
                    let reference_image_urls = reference_image_provider_inputs_by_ids(
                        video,
                        self.state.data_dir(),
                        jwt_secret,
                        input.user_id,
                        input.project_id,
                        input.chapter_id,
                        input.reference_image_ids,
                        &mut staging_artifacts,
                        input.transport,
                    )
                    .await
                    .map_err(host_error)?;
                    let first_frame_image_url = reference_image_provider_input_by_id(
                        video,
                        self.state.data_dir(),
                        jwt_secret,
                        input.user_id,
                        input.project_id,
                        input.chapter_id,
                        input.first_frame_image_id,
                        &mut staging_artifacts,
                        input.transport,
                    )
                    .await
                    .map_err(host_error)?;
                    let last_frame_image_url = reference_image_provider_input_by_id(
                        video,
                        self.state.data_dir(),
                        jwt_secret,
                        input.user_id,
                        input.project_id,
                        input.chapter_id,
                        input.last_frame_image_id,
                        &mut staging_artifacts,
                        input.transport,
                    )
                    .await
                    .map_err(host_error)?;
                    return Ok(ResolvedChapterVideoImageInputs {
                        reference_image_urls,
                        first_frame_image_url,
                        last_frame_image_url,
                        staging_artifacts,
                    });
                }
                let mut reference_staging_artifacts = Vec::new();
                let mut first_frame_staging_artifacts = Vec::new();
                let mut last_frame_staging_artifacts = Vec::new();
                let (reference_result, first_frame_result, last_frame_result) = futures::join!(
                    reference_image_provider_inputs_by_ids(
                        video,
                        self.state.data_dir(),
                        jwt_secret,
                        input.user_id,
                        input.project_id,
                        input.chapter_id,
                        input.reference_image_ids,
                        &mut reference_staging_artifacts,
                        input.transport,
                    ),
                    reference_image_provider_input_by_id(
                        video,
                        self.state.data_dir(),
                        jwt_secret,
                        input.user_id,
                        input.project_id,
                        input.chapter_id,
                        input.first_frame_image_id,
                        &mut first_frame_staging_artifacts,
                        input.transport,
                    ),
                    reference_image_provider_input_by_id(
                        video,
                        self.state.data_dir(),
                        jwt_secret,
                        input.user_id,
                        input.project_id,
                        input.chapter_id,
                        input.last_frame_image_id,
                        &mut last_frame_staging_artifacts,
                        input.transport,
                    ),
                );
                let reference_image_urls = reference_result.map_err(host_error)?;
                let first_frame_image_url = first_frame_result.map_err(host_error)?;
                let last_frame_image_url = last_frame_result.map_err(host_error)?;
                let mut staging_artifacts = reference_staging_artifacts;
                staging_artifacts.extend(first_frame_staging_artifacts);
                staging_artifacts.extend(last_frame_staging_artifacts);
                Ok(ResolvedChapterVideoImageInputs {
                    reference_image_urls,
                    first_frame_image_url,
                    last_frame_image_url,
                    staging_artifacts,
                })
            }
            VideoProviderValidationMode::Preview => {
                let (reference_result, first_frame_result, last_frame_result) = futures::join!(
                    reference_image_request_preview_urls_by_ids(
                        video,
                        input.user_id,
                        input.project_id,
                        input.chapter_id,
                        input.reference_image_ids,
                    ),
                    reference_image_request_preview_url_by_id(
                        video,
                        input.user_id,
                        input.project_id,
                        input.chapter_id,
                        input.first_frame_image_id,
                    ),
                    reference_image_request_preview_url_by_id(
                        video,
                        input.user_id,
                        input.project_id,
                        input.chapter_id,
                        input.last_frame_image_id,
                    ),
                );
                let reference_image_urls = reference_result.map_err(host_error)?;
                let first_frame_image_url = first_frame_result.map_err(host_error)?;
                let last_frame_image_url = last_frame_result.map_err(host_error)?;
                Ok(ResolvedChapterVideoImageInputs {
                    reference_image_urls,
                    first_frame_image_url,
                    last_frame_image_url,
                    staging_artifacts: Vec::new(),
                })
            }
        }
    }

    fn redacted_runtime_config_snapshot(&self) -> Option<Value> {
        crate::runtime_config::redacted_current_snapshot_value()
    }
}

fn video_cancellation_handle(token: CancellationToken) -> VideoCancellationHandle {
    let cancelled = token;
    VideoCancellationHandle::new(move || {
        let token = cancelled.clone();
        Box::pin(async move { token.cancelled().await })
    })
}

#[async_trait]
impl VideoProviderHostPort for RuntimeVideoProviderHostPort<'_> {
    fn inspect_video_file(
        &self,
        relative_path: &str,
        label: &str,
    ) -> Result<GeneratedMediaArtifactFile, AppError> {
        inspect_generated_media_file(
            self.state.data_dir(),
            relative_path,
            label,
            video_mime_from_path,
        )
        .map_err(host_error)
    }

    async fn poll_video_job(
        &self,
        project_id: i64,
        workflow_run_id: i64,
        config: &VideoProviderConfig,
        job_id: &str,
        cancel_token: &VideoCancellationHandle,
    ) -> Result<VideoJobState, AppError> {
        let root_project_id = video_sidecar_root_project_id(
            self.state.runtime_profile(),
            self.state.project_access_application(),
            project_id,
        )
        .await
        .map_err(host_error)?;
        let provider_cancel = CancellationToken::new();
        let poll = poll_provider_job(
            self.state,
            config,
            project_id,
            root_project_id,
            job_id,
            TaskPriority::Low,
            Some(workflow_run_id),
            provider_cancel.clone(),
        );
        tokio::pin!(poll);
        let result = tokio::select! {
            result = &mut poll => result,
            _ = cancel_token.cancelled() => {
                provider_cancel.cancel();
                (&mut poll).await
            }
        };
        result.map_err(host_error)
    }

    async fn persist_video_result(
        &self,
        target: &VideoProviderResultTarget,
        _prompt_id: Option<i64>,
        mut result: VideoGenerationResult,
    ) -> Result<PersistedVideoResult, AppError> {
        result.metadata = super::preview::completed_video_artifact_metadata(&result);
        if is_canvas_video_record_fields(target.chapter_id, target.workflow_mode.as_deref()) {
            let persisted = self
                .persist_canvas_video_file(CanvasVideoFileInput {
                    project_id: target.project_id,
                    video_id: target.video_id,
                    bytes: &result.bytes,
                    format: &result.format,
                })
                .await?;
            return Ok(PersistedVideoResult {
                video_id: target.video_id,
                project_id: target.project_id,
                file_path: persisted.relative_path,
                thumbnail_path: None,
                properties: persisted.properties,
                metadata_json: Some(json!({
                    "provider": result.metadata,
                    "canvas_video": true,
                })),
            });
        }
        super::job_lifecycle::persist_video_result(self.state, target, result)
            .await
            .map_err(host_error)
    }

    async fn remove_video_result(&self, persisted: &PersistedVideoResult) -> Result<(), AppError> {
        // 先写 durable cleanup intent，再做同步尾帧清理。
        // 进程若在 DB 失败窗口崩溃，outbox 仍可跨重启删除正式视频文件。
        let mut paths = vec![persisted.file_path.clone()];
        if let Some(path) = persisted.thumbnail_path.clone() {
            paths.push(path);
        }
        enqueue_file_artifact_cleanup_by_project(
            self.state,
            &format!(
                "video-provider-result:{}:{}",
                persisted.project_id, persisted.video_id
            ),
            &paths,
        )
        .await
        .map_err(host_error)?;
        if let Err(error) = delete_video_tail_frame_assets(self.state, persisted.video_id).await {
            return Err(host_error(format!(
                "视频文件 cleanup intent 已写入，但尾帧同步清理失败：{error}"
            )));
        }
        Ok(())
    }
}

#[async_trait]
impl ReferenceMediaVideoPreviewHostPort for RuntimeVideoProviderHostPort<'_> {
    async fn resolve_video_config_for_preview(
        &self,
        config_id: Option<i64>,
    ) -> Result<VideoProviderConfig, AppError> {
        self.resolve_config(config_id).await
    }

    async fn reference_media_provider_request_body_for_preview(
        &self,
        input: VideoProviderRequestInput,
    ) -> Result<VideoProviderRequestBody, AppError> {
        self.provider_request_body(input).await
    }

    async fn upload_reference_media_input_for_preview(
        &self,
        owner_user_id: i64,
        project_id: i64,
        source_media_id: i64,
        source_media_kind: &str,
        file_name: &str,
        mime_type: &str,
        bytes: &[u8],
    ) -> Result<(String, GenerationArtifactRef), AppError> {
        let uploaded = media_bytes_to_remote_temp_media_url(
            a4n_api_contract::temp_media::TempMediaUploadRequest {
                owner_user_id,
                project_id,
                source_media_kind: source_media_kind.to_string(),
                source_media_id,
                file_name: file_name.to_string(),
                mime_type: mime_type.to_string(),
            },
            bytes.to_vec(),
        )
        .await
        .map_err(host_error)?;
        Ok((uploaded.url, uploaded.artifact))
    }
}

#[async_trait]
impl ReferenceMediaVideoGenerationHostPort for RuntimeVideoProviderHostPort<'_> {
    async fn resolve_video_config(
        &self,
        config_id: Option<i64>,
    ) -> Result<VideoProviderConfig, AppError> {
        self.resolve_config(config_id).await
    }

    async fn reference_media_provider_request_body(
        &self,
        input: VideoProviderRequestInput,
    ) -> Result<VideoProviderRequestBody, AppError> {
        <Self as ReferenceMediaVideoPreviewHostPort>::reference_media_provider_request_body_for_preview(
            self, input,
        )
        .await
    }

    async fn submit_reference_media_video(
        &self,
        input: VideoProviderRequestInput,
        progress: &mut dyn VideoGenerationProgressPort,
    ) -> Result<VideoGenerationResult, AppError> {
        self.submit_video(input, progress).await
    }

    async fn upload_reference_media_input(
        &self,
        owner_user_id: i64,
        project_id: i64,
        source_media_id: i64,
        source_media_kind: &str,
        file_name: &str,
        mime_type: &str,
        bytes: &[u8],
    ) -> Result<(String, GenerationArtifactRef), AppError> {
        <Self as ReferenceMediaVideoPreviewHostPort>::upload_reference_media_input_for_preview(
            self,
            owner_user_id,
            project_id,
            source_media_id,
            source_media_kind,
            file_name,
            mime_type,
            bytes,
        )
        .await
    }

    fn runtime_config_snapshot(&self) -> Option<Value> {
        self.redacted_runtime_config_snapshot()
    }

    fn video_provider_host_port(&self) -> &dyn VideoProviderHostPort {
        self
    }
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
                chapter_id: 0,
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

#[async_trait]
impl ChapterVideoPreviewHostPort for RuntimeVideoProviderHostPort<'_> {
    async fn resolve_video_config_for_preview(
        &self,
        config_id: Option<i64>,
    ) -> Result<VideoProviderConfig, AppError> {
        self.resolve_config(config_id).await
    }

    fn resolve_image_transport_for_preview(
        &self,
        backend: &str,
    ) -> Result<ProviderImageInputTransport, AppError> {
        self.resolve_image_transport_setting(backend)
    }

    async fn resolve_chapter_video_image_inputs_for_preview(
        &self,
        input: ChapterVideoImageInput,
    ) -> Result<ResolvedChapterVideoImageInputs, AppError> {
        self.resolve_video_image_inputs(input).await
    }

    async fn chapter_video_provider_request_body_for_preview(
        &self,
        input: VideoProviderRequestInput,
    ) -> Result<VideoProviderRequestBody, AppError> {
        self.provider_request_body(input).await
    }
}

#[async_trait]
impl ChapterVideoGenerationHostPort for RuntimeVideoProviderHostPort<'_> {
    async fn resolve_video_config(
        &self,
        config_id: Option<i64>,
    ) -> Result<VideoProviderConfig, AppError> {
        self.resolve_config(config_id).await
    }

    async fn resolve_chapter_video_image_inputs(
        &self,
        input: ChapterVideoImageInput,
    ) -> Result<ResolvedChapterVideoImageInputs, AppError> {
        <Self as ChapterVideoPreviewHostPort>::resolve_chapter_video_image_inputs_for_preview(
            self, input,
        )
        .await
    }

    async fn chapter_video_provider_request_body(
        &self,
        input: VideoProviderRequestInput,
    ) -> Result<VideoProviderRequestBody, AppError> {
        <Self as ChapterVideoPreviewHostPort>::chapter_video_provider_request_body_for_preview(
            self, input,
        )
        .await
    }

    async fn submit_chapter_video(
        &self,
        input: VideoProviderRequestInput,
        progress: &mut dyn VideoGenerationProgressPort,
    ) -> Result<VideoGenerationResult, AppError> {
        self.submit_video(input, progress).await
    }

    fn runtime_config_snapshot(&self) -> Option<Value> {
        self.redacted_runtime_config_snapshot()
    }

    fn video_provider_host_port(&self) -> &dyn VideoProviderHostPort {
        self
    }
}

pub(super) fn host_error(message: String) -> AppError {
    AppError::ConfigError(message)
}
