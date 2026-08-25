use a4n_core::{
    config::env::optional_non_empty_env_string,
    error::AppError,
    media::video_mime_from_path,
    models::ai::{generated_video, video_gen_config},
};
use a4n_orchestrator::canvas::{
    CanvasVideoFileInput, CanvasVideoGenerationHostPort, CanvasVideoImageInput,
    CanvasVideoPersistedFile, CanvasVideoProviderInput, CanvasVideoResolvedImages,
};
use a4n_orchestrator::generation::GeneratedMediaArtifactFile;
use a4n_orchestrator::video::{
    ChapterVideoGenerationHostPort, ChapterVideoImageInput, PersistedVideoResult,
    ProviderImageInputTransport, ReferenceMediaVideoGenerationHostPort,
    ResolvedChapterVideoImageInputs, VideoGenerationProgress, VideoProviderHostPort,
    VideoProviderRequestBody, VideoProviderRequestInput, VideoProviderValidationMode,
    append_video_request_prompt_length_context, is_canvas_video_record,
    provider_image_input_transport_with_override, video_request_prompt_length_context,
};
use a4n_queue::priority::{TaskKind as QueueTaskKind, TaskPriority};
use a4n_service_protocol::GenerationArtifactRef;
#[cfg(feature = "embedded-media-providers")]
use a4n_video::video_provider_capabilities;
use a4n_video::{VideoGenerationResult, VideoJobState};
use async_trait::async_trait;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use tokio_util::sync::CancellationToken;

#[cfg(feature = "embedded-media-providers")]
use a4n_video_providers::video_provider_request_body_preview;

use crate::actions::image_generation::run_queued_request_with_cancel_and_workflow;
use crate::{
    AppState,
    actions::video_generation::{
        job_lifecycle::poll_provider_job,
        media::{
            delete_video_tail_frame_assets, media_bytes_to_remote_temp_media_url,
            persist_canvas_video_bytes, reference_image_provider_input_by_id,
            reference_image_provider_inputs_by_ids, reference_image_request_preview_url_by_id,
            reference_image_request_preview_urls_by_ids,
        },
        relay_video_sidecar_progress, resolve_video_config, run_queued_request_with_cancel,
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
    ) -> Result<video_gen_config::Model, AppError> {
        resolve_video_config(self.state.orchestrator(), config_id)
            .await
            .map_err(host_error)
    }

    async fn provider_request_body(
        &self,
        input: VideoProviderRequestInput,
    ) -> Result<VideoProviderRequestBody, AppError> {
        if self.state.runtime_profile().uses_sidecars() {
            let collaboration = self.state.orchestrator();
            let root_project_id = video_sidecar_root_project_id(
                self.state.runtime_profile(),
                collaboration,
                input.project_id,
            )
            .await
            .map_err(host_error)?;
            let request = crate::sidecar_clients::preview_video_provider_request(
                self.state.capabilities(),
                &input.config,
                input.request,
                Some(input.project_id),
                root_project_id,
            )
            .await
            .map_err(host_error)?;
            return Ok(VideoProviderRequestBody {
                request,
                source_label: "由 video sidecar 生成真实 provider 请求体".to_string(),
            });
        }

        #[cfg(feature = "embedded-media-providers")]
        {
            let capabilities =
                video_provider_capabilities(&input.config.backend).ok_or_else(|| {
                    host_error(format!(
                        "未知视频 backend={}，无法展示真实 provider 请求体",
                        input.config.backend
                    ))
                })?;
            let request = video_provider_request_body_preview(
                &input.config.backend,
                input
                    .config
                    .base_url
                    .as_deref()
                    .ok_or_else(|| host_error("视频生成配置缺少 base_url".to_string()))?,
                &input.request,
            )
            .map_err(|error| host_error(error.to_string()))?;
            return Ok(VideoProviderRequestBody {
                request,
                source_label: format!(
                    "由 embedded {} provider 生成真实请求体",
                    capabilities.display_name
                ),
            });
        }

        #[cfg(not(feature = "embedded-media-providers"))]
        Err(host_error(format!(
            "视频 provider 请求不可用：backend={}；当前 runtime 既未启用 sidecar profile，也未编译 embedded-media-providers",
            input.config.backend
        )))
    }

    async fn submit_video(
        &self,
        input: VideoProviderRequestInput,
        progress: &mut VideoGenerationProgress<'_>,
    ) -> Result<VideoGenerationResult, AppError> {
        let collaboration = self.state.orchestrator();
        let root_project_id = video_sidecar_root_project_id(
            self.state.runtime_profile(),
            collaboration,
            input.project_id,
        )
        .await
        .map_err(host_error)?;
        let prompt_length_context = video_request_prompt_length_context(&input.request);
        let (sidecar_progress_tx, sidecar_progress_rx) = tokio::sync::mpsc::unbounded_channel();
        let request_future = run_queued_request_with_cancel(
            self.state,
            QueueTaskKind::VideoGeneration,
            TaskPriority::Medium,
            Some(input.project_id),
            progress.cancel_token(),
            {
                let state = self.state;
                move || async move {
                    crate::sidecar_clients::generate_video(
                        state.runtime_profile(),
                        state.capabilities(),
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
        relay_video_sidecar_progress(progress, sidecar_progress_rx, request_future)
            .await
            .map_err(|error| {
                host_error(append_video_request_prompt_length_context(
                    error,
                    &prompt_length_context,
                ))
            })
    }

    async fn resolve_video_image_inputs(
        &self,
        input: ChapterVideoImageInput,
    ) -> Result<ResolvedChapterVideoImageInputs, AppError> {
        let video = self.state.orchestrator();
        let collaboration = self.state.orchestrator();
        match input.mode {
            VideoProviderValidationMode::Generate => {
                let mut staging_artifacts = Vec::new();
                let jwt_secret = if input.transport == ProviderImageInputTransport::PublicUrl {
                    Some(self.state.orchestrator().required_jwt_secret()?)
                } else {
                    None
                };
                Ok(ResolvedChapterVideoImageInputs {
                    reference_image_urls: reference_image_provider_inputs_by_ids(
                        video,
                        collaboration,
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
                    .map_err(host_error)?,
                    first_frame_image_url: reference_image_provider_input_by_id(
                        video,
                        collaboration,
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
                    .map_err(host_error)?,
                    last_frame_image_url: reference_image_provider_input_by_id(
                        video,
                        collaboration,
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
                    .map_err(host_error)?,
                    staging_artifacts,
                })
            }
            VideoProviderValidationMode::Preview => {
                let reference_image_urls = reference_image_request_preview_urls_by_ids(
                    video,
                    collaboration,
                    input.user_id,
                    input.project_id,
                    input.chapter_id,
                    input.reference_image_ids,
                )
                .await
                .map_err(host_error)?;
                let first_frame_image_url = reference_image_request_preview_url_by_id(
                    video,
                    collaboration,
                    input.user_id,
                    input.project_id,
                    input.chapter_id,
                    input.first_frame_image_id,
                )
                .await
                .map_err(host_error)?;
                let last_frame_image_url = reference_image_request_preview_url_by_id(
                    video,
                    collaboration,
                    input.user_id,
                    input.project_id,
                    input.chapter_id,
                    input.last_frame_image_id,
                )
                .await
                .map_err(host_error)?;
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
        config: &video_gen_config::Model,
        job_id: &str,
        cancel_token: CancellationToken,
    ) -> Result<VideoJobState, AppError> {
        let collaboration = self.state.orchestrator();
        let root_project_id =
            video_sidecar_root_project_id(self.state.runtime_profile(), collaboration, project_id)
                .await
                .map_err(host_error)?;
        poll_provider_job(
            self.state,
            config,
            project_id,
            root_project_id,
            job_id,
            TaskPriority::Low,
            Some(workflow_run_id),
            cancel_token,
        )
        .await
        .map_err(host_error)
    }

    async fn persist_video_result(
        &self,
        video: &generated_video::Model,
        _prompt_id: Option<i64>,
        mut result: VideoGenerationResult,
    ) -> Result<PersistedVideoResult, AppError> {
        result.metadata = completed_video_artifact_metadata(&result);
        if is_canvas_video_record(video) {
            let (file_path, file_size) = persist_canvas_video_bytes(
                self.state.data_dir(),
                video.project_id,
                video.id,
                &result.bytes,
                &result.format,
            )
            .map_err(host_error)?;
            return Ok(PersistedVideoResult {
                video_id: video.id,
                project_id: video.project_id,
                file_path,
                thumbnail_path: None,
                file_size,
                metadata_json: Some(json!({
                    "provider": result.metadata,
                    "canvas_video": true,
                })),
            });
        }
        super::job_lifecycle::persist_video_result(self.state, video, result)
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
        self.state
            .orchestrator()
            .insert_file_artifact_cleanup_intent(
                persisted.project_id,
                format!(
                    "video-provider-result:{}:{}",
                    persisted.project_id, persisted.video_id
                ),
                paths,
            )
            .await
            .map_err(|error| host_error(format!("写入视频文件 cleanup intent 失败：{error}")))?;
        if let Err(error) = delete_video_tail_frame_assets(self.state, persisted.video_id).await {
            return Err(host_error(format!(
                "视频文件 cleanup intent 已写入，但尾帧同步清理失败：{error}"
            )));
        }
        Ok(())
    }
}

pub(super) fn completed_video_artifact_metadata(result: &VideoGenerationResult) -> Value {
    json!({
        "provider_job_id": result.provider_job_id,
        "artifact": {
            "format": result.format,
            "byte_size": result.bytes.len(),
            "sha256": format!("{:x}", Sha256::digest(&result.bytes)),
        },
    })
}

#[async_trait]
impl ReferenceMediaVideoGenerationHostPort for RuntimeVideoProviderHostPort<'_> {
    async fn resolve_video_config(
        &self,
        config_id: Option<i64>,
    ) -> Result<video_gen_config::Model, AppError> {
        self.resolve_config(config_id).await
    }

    async fn reference_media_provider_request_body(
        &self,
        input: VideoProviderRequestInput,
    ) -> Result<VideoProviderRequestBody, AppError> {
        self.provider_request_body(input).await
    }

    async fn submit_reference_media_video(
        &self,
        input: VideoProviderRequestInput,
        progress: &mut VideoGenerationProgress<'_>,
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
        let uploaded = media_bytes_to_remote_temp_media_url(
            owner_user_id,
            project_id,
            source_media_kind,
            source_media_id,
            file_name,
            mime_type,
            bytes,
        )
        .await
        .map_err(host_error)?;
        Ok((uploaded.url, uploaded.artifact))
    }

    fn runtime_config_snapshot(&self) -> Option<Value> {
        self.redacted_runtime_config_snapshot()
    }

    fn video_provider_host_port(&self) -> &dyn VideoProviderHostPort {
        self
    }
}

#[async_trait]
impl CanvasVideoGenerationHostPort for RuntimeVideoProviderHostPort<'_> {
    async fn resolve_video_config(
        &self,
        config_id: Option<i64>,
    ) -> Result<video_gen_config::Model, AppError> {
        self.resolve_config(config_id).await
    }

    async fn resolve_canvas_video_images(
        &self,
        input: CanvasVideoImageInput,
    ) -> Result<CanvasVideoResolvedImages, AppError> {
        let transport = ChapterVideoGenerationHostPort::resolve_image_transport(self)?;
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

    async fn submit_canvas_video(
        &self,
        input: CanvasVideoProviderInput,
    ) -> Result<VideoGenerationResult, AppError> {
        let collaboration = self.state.orchestrator();
        let root_project_id = video_sidecar_root_project_id(
            self.state.runtime_profile(),
            collaboration,
            input.project_id,
        )
        .await
        .map_err(host_error)?;
        run_queued_request_with_cancel_and_workflow(
            self.state,
            QueueTaskKind::VideoGeneration,
            TaskPriority::Medium,
            Some(input.project_id),
            Some(input.workflow_run_id),
            CancellationToken::new(),
            || async move {
                crate::sidecar_clients::generate_video(
                    self.state.runtime_profile(),
                    self.state.capabilities(),
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
        .await
        .map_err(host_error)
    }

    async fn poll_canvas_video(
        &self,
        project_id: i64,
        workflow_run_id: i64,
        config: &video_gen_config::Model,
        job_id: &str,
    ) -> Result<VideoJobState, AppError> {
        <Self as VideoProviderHostPort>::poll_video_job(
            self,
            project_id,
            workflow_run_id,
            config,
            job_id,
            CancellationToken::new(),
        )
        .await
    }

    fn persist_canvas_video_file(
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
        Ok(CanvasVideoPersistedFile {
            relative_path,
            file_size,
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
impl ChapterVideoGenerationHostPort for RuntimeVideoProviderHostPort<'_> {
    async fn resolve_video_config(
        &self,
        config_id: Option<i64>,
    ) -> Result<video_gen_config::Model, AppError> {
        self.resolve_config(config_id).await
    }

    fn resolve_image_transport(&self) -> Result<ProviderImageInputTransport, AppError> {
        let override_value = optional_non_empty_env_string("A4N_VIDEO_IMAGE_INPUT_TRANSPORT")?;
        provider_image_input_transport_with_override(override_value.as_deref()).map_err(host_error)
    }

    async fn resolve_chapter_video_image_inputs(
        &self,
        input: ChapterVideoImageInput,
    ) -> Result<ResolvedChapterVideoImageInputs, AppError> {
        self.resolve_video_image_inputs(input).await
    }

    async fn chapter_video_provider_request_body(
        &self,
        input: VideoProviderRequestInput,
    ) -> Result<VideoProviderRequestBody, AppError> {
        self.provider_request_body(input).await
    }

    async fn submit_chapter_video(
        &self,
        input: VideoProviderRequestInput,
        progress: &mut VideoGenerationProgress<'_>,
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

fn host_error(message: String) -> AppError {
    AppError::InvalidState {
        current: "video_provider_host".to_string(),
        operation: message,
    }
}
