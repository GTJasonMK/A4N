use a4n_core::{
    error::AppError,
    models::ai::{generated_video, video_gen_config},
    video_execution::VideoSegmentWorkflowModeV1,
};
use a4n_db::collaboration::ProjectAccessRole;
use a4n_service_protocol::GenerationArtifactRef;
use a4n_video::{VideoGenerationRequest, VideoGenerationResult};
use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::OrchestratorContext;

use super::{
    CLIPROXY_OPENAI_VIDEO_BACKEND, SEEDANCE2_VIDEO_BACKEND, VIDEO_GENERATION_PROGRESS_TOTAL,
    VideoGenerationProgress, VideoGenerationRecordInput, VideoProviderExecutionInput,
    VideoProviderHostPort, VideoProviderRequestBody, VideoProviderRequestInput,
    VideoProviderValidationMode, provider_request_snapshot_value,
    reference_media_rules::{
        ReferenceMediaVideoInput, ResolvedReferenceMediaVideoInput, ValidatedReferenceMediaUpload,
        build_reference_media_video_request, reference_media_prompt_from_recipe,
        reference_media_provider_request_note, resolve_reference_media_urls,
        validate_reference_media_start_input, validated_reference_media_uploads,
    },
};

#[derive(Debug, Clone, Deserialize)]
pub struct GenerateReferenceMediaVideoInput {
    pub user_id: i64,
    pub project_id: i64,
    pub chapter_id: i64,
    pub input: ReferenceMediaVideoInput,
}

#[derive(Debug, Clone)]
pub struct ReferenceMediaVideoRequestPreview {
    pub request: Value,
    pub note: String,
}

#[async_trait]
pub trait ReferenceMediaVideoGenerationHostPort: Send + Sync {
    async fn resolve_video_config(
        &self,
        config_id: Option<i64>,
    ) -> Result<video_gen_config::Model, AppError>;

    async fn reference_media_provider_request_body(
        &self,
        input: VideoProviderRequestInput,
    ) -> Result<VideoProviderRequestBody, AppError>;

    async fn submit_reference_media_video(
        &self,
        input: VideoProviderRequestInput,
        progress: &mut VideoGenerationProgress<'_>,
    ) -> Result<VideoGenerationResult, AppError>;

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
        let _ = (
            owner_user_id,
            project_id,
            source_media_id,
            source_media_kind,
            file_name,
            mime_type,
            bytes,
        );
        Err(AppError::InvalidArgument(
            "当前运行环境不支持本地参考媒体临时上传".to_string(),
        ))
    }

    fn runtime_config_snapshot(&self) -> Option<Value>;

    fn video_provider_host_port(&self) -> &dyn VideoProviderHostPort;
}

struct PreparedReferenceMediaVideoGeneration {
    project_id: i64,
    chapter_id: i64,
    segment_plan_prompt_id: i64,
    shot_number: i32,
    video_execution_identity: Value,
    config: video_gen_config::Model,
    request: VideoGenerationRequest,
    redacted_local_uploads: usize,
    staging_artifacts: Vec<GenerationArtifactRef>,
}

impl OrchestratorContext {
    pub async fn preview_reference_media_video_request_validated(
        &self,
        host_port: &dyn ReferenceMediaVideoGenerationHostPort,
        input: GenerateReferenceMediaVideoInput,
    ) -> Result<ReferenceMediaVideoRequestPreview, AppError> {
        let prepared = self
            .prepare_reference_media_video_generation(
                host_port,
                input,
                VideoProviderValidationMode::Preview,
            )
            .await?;
        let body = host_port
            .reference_media_provider_request_body(VideoProviderRequestInput {
                project_id: prepared.project_id,
                config: prepared.config.clone(),
                request: prepared.request.clone(),
                provider_idempotency_key_sha256: None,
            })
            .await?;
        let source_label = if prepared.redacted_local_uploads == 0 {
            format!("当前片段执行方案；{}", body.source_label)
        } else {
            format!(
                "当前片段执行方案；{}；已校验的 {} 个本地媒体在预览中以脱敏 URL 表示",
                body.source_label, prepared.redacted_local_uploads
            )
        };
        let note = reference_media_provider_request_note(
            &source_label,
            &prepared.config,
            &prepared.request,
        )
        .map_err(AppError::InvalidArgument)?;
        Ok(ReferenceMediaVideoRequestPreview {
            request: body.request,
            note,
        })
    }

    pub async fn run_reference_media_video_generation_validated(
        &self,
        host_port: &dyn ReferenceMediaVideoGenerationHostPort,
        input: GenerateReferenceMediaVideoInput,
        progress: &mut VideoGenerationProgress<'_>,
    ) -> Result<generated_video::Model, AppError> {
        progress.progress(1, VIDEO_GENERATION_PROGRESS_TOTAL, "正在校验参考媒体请求")?;
        progress.check_cancelled("参考媒体视频生成已停止")?;
        let prepared = self
            .prepare_reference_media_video_generation(
                host_port,
                input,
                VideoProviderValidationMode::Generate,
            )
            .await?;

        progress.progress(2, VIDEO_GENERATION_PROGRESS_TOTAL, "正在创建视频生成记录")?;
        progress.check_cancelled("参考媒体视频生成已停止")?;
        let request_body = host_port
            .reference_media_provider_request_body(VideoProviderRequestInput {
                project_id: prepared.project_id,
                config: prepared.config.clone(),
                request: prepared.request.clone(),
                provider_idempotency_key_sha256: None,
            })
            .await?;
        let snapshot_note = reference_media_provider_request_note(
            &format!("生成时请求快照；{}", request_body.source_label),
            &prepared.config,
            &prepared.request,
        )
        .map_err(AppError::InvalidArgument)?;
        let provider_request_snapshot = provider_request_snapshot_value(
            "reference_media_video",
            "参考媒体视频 provider 请求",
            "video_provider_request",
            Some("reference_media_video"),
            Some(snapshot_note),
            request_body.request,
        );
        let a4n_video::VideoMediaInput::References {
            images,
            videos,
            audio,
        } = &prepared.request.media
        else {
            return Err(AppError::InvalidArgument(
                "参考媒体请求要求 media.type=references".into(),
            ));
        };
        let provider_reference_mode = match prepared.config.backend.as_str() {
            SEEDANCE2_VIDEO_BACKEND => "omni_reference",
            CLIPROXY_OPENAI_VIDEO_BACKEND => "image_reference",
            backend => {
                return Err(AppError::InvalidArgument(format!(
                    "参考媒体请求不支持 backend={backend}"
                )));
            }
        };
        let metadata = json!({
            "segment_plan_prompt_id": prepared.segment_plan_prompt_id,
            "shot_number": prepared.shot_number,
            "video_execution_identity": prepared.video_execution_identity,
            "config_id": prepared.config.id,
            "provider_backend": prepared.config.backend,
            "reference_media": {
                "functionMode": provider_reference_mode,
                "image_file_count": images.len(),
                "video_file_count": videos.len(),
                "audio_file_count": audio.len(),
                "image_files": images,
                "video_files": videos,
                "audio_files": audio,
            },
            "provider_request_snapshot": provider_request_snapshot,
            "runtime_config_snapshot": host_port.runtime_config_snapshot(),
        });
        let video = self
            .create_video_generation_record_validated(VideoGenerationRecordInput {
                project_id: prepared.project_id,
                chapter_id: Some(prepared.chapter_id),
                video_type: generated_video::VIDEO_TYPE_REFERENCE_MEDIA.into(),
                workflow_mode: Some(
                    VideoSegmentWorkflowModeV1::ReferenceMedia
                        .as_str()
                        .to_string(),
                ),
                prompt: prepared.request.prompt.clone(),
                negative_prompt: None,
                model: prepared.config.model.clone(),
                width: prepared.request.width,
                height: prepared.request.height,
                duration_secs: prepared.request.duration_secs as f32,
                fps: prepared.request.fps,
                source_video_id: None,
                source_image_id: None,
                metadata_json: Some(metadata),
                provider_request_snapshot,
                provider_backend: prepared.config.backend.clone(),
                step_key: "generate_reference_media_video".to_string(),
                title: "参考媒体视频 provider 生成".to_string(),
                prompt_template_name: Some("reference_media_video".to_string()),
                // 共享片段规划可生成多个候选视频，不参与一对一 prompt 生命周期。
                prompt_id: None,
            })
            .await?;
        if let Err(error) = self
            .ensure_created_video_capacity(
                prepared.config.id,
                prepared.config.max_concurrent,
                video.id,
            )
            .await
        {
            return self
                .mark_generated_video_failed_validated(video.id, None, &error.to_string())
                .await;
        }

        progress.progress(
            3,
            VIDEO_GENERATION_PROGRESS_TOTAL,
            "正在调用参考媒体视频模型",
        )?;
        if progress.check_cancelled("参考媒体视频生成已停止").is_err() {
            self.mark_generated_video_cancelled_validated(video.id, None, "参考媒体视频生成已停止")
                .await?;
            return Err(AppError::Cancelled);
        }
        let mut provider_input = VideoProviderRequestInput {
            project_id: prepared.project_id,
            config: prepared.config.clone(),
            request: prepared.request,
            provider_idempotency_key_sha256: None,
        };
        provider_input.provider_idempotency_key_sha256 = progress
            .checkpoint_provider_request(
                video.id,
                &provider_input.config,
                &provider_input.request,
                &prepared.staging_artifacts,
            )
            .await?;
        let result = host_port
            .submit_reference_media_video(provider_input, progress)
            .await
            .map_err(|error| error.to_string());
        if let Ok(result) = &result
            && let Some(job_id) = result
                .provider_job_id
                .as_deref()
                .filter(|id| !id.trim().is_empty())
        {
            progress
                .checkpoint_provider_job(video.id, &prepared.config, job_id)
                .await?;
        }
        if result.as_ref().is_ok_and(|result| result.bytes.is_empty()) {
            progress.progress(
                4,
                VIDEO_GENERATION_PROGRESS_TOTAL,
                "参考媒体视频任务已提交，开始等待生成结果",
            )?;
        } else {
            progress.progress(
                6,
                VIDEO_GENERATION_PROGRESS_TOTAL,
                "视频模型已返回，正在保存文件",
            )?;
        }
        let updated = self
            .apply_video_provider_result_validated(
                VideoProviderExecutionInput {
                    video,
                    config: prepared.config,
                    prompt_id: None,
                    missing_result_message: "参考媒体视频模型未返回文件或任务 ID".to_string(),
                    cancel_token: progress.cancel_token(),
                },
                result,
                host_port.video_provider_host_port(),
            )
            .await?;
        progress.progress(7, VIDEO_GENERATION_PROGRESS_TOTAL, "视频文件已保存")?;
        Ok(updated)
    }

    async fn prepare_reference_media_video_generation(
        &self,
        host_port: &dyn ReferenceMediaVideoGenerationHostPort,
        input: GenerateReferenceMediaVideoInput,
        mode: VideoProviderValidationMode,
    ) -> Result<PreparedReferenceMediaVideoGeneration, AppError> {
        validate_reference_media_start_input(&input.input).map_err(AppError::InvalidArgument)?;
        let GenerateReferenceMediaVideoInput {
            user_id,
            project_id,
            chapter_id,
            input,
        } = input;
        let (operation, writable) = match mode {
            VideoProviderValidationMode::Generate => ("生成参考媒体视频", true),
            VideoProviderValidationMode::Preview => ("预览参考媒体视频请求", false),
        };
        self.verify_project_permission(project_id, user_id, ProjectAccessRole::Editor)
            .await?;
        self.project_chapter_video_source(project_id, chapter_id, operation, writable)
            .await?;
        let segment_plan_prompt_id = input.segment_plan_prompt_id;
        let plan = self
            .require_current_shared_segment_plan(project_id, chapter_id, segment_plan_prompt_id)
            .await?;
        let shot_number = a4n_core::video_execution::resolve_video_segment_number(
            plan.metadata_json.as_ref(),
            &input.target_segment.source_shot_id,
            input.target_segment.segment_index_in_shot,
        )
        .map_err(AppError::InvalidArgument)?;
        let recipe = a4n_core::video_execution::video_segment_recipe_from_metadata(
            plan.metadata_json.as_ref(),
            shot_number,
        )
        .map_err(AppError::InvalidArgument)?
        .ok_or_else(|| {
            AppError::InvalidArgument(format!("当前规划缺少片段 {shot_number} 的执行方案"))
        })?;
        if recipe.workflow_mode != VideoSegmentWorkflowModeV1::ReferenceMedia {
            return Err(AppError::InvalidArgument(format!(
                "片段 {shot_number} 当前未选择参考媒体方案"
            )));
        }
        let reference_media = recipe.reference_media.as_ref().ok_or_else(|| {
            AppError::InvalidArgument(format!("片段 {shot_number} 尚未保存参考媒体提示词和媒体槽"))
        })?;
        let plan_identity = plan
            .metadata_json
            .as_ref()
            .and_then(|metadata| metadata.get("video_execution_identity"))
            .cloned()
            .ok_or_else(|| AppError::InvalidArgument("当前片段规划缺少执行身份".to_string()))?;
        let video_execution_identity = super::prompt_generation::video_segment_execution_identity(
            plan_identity,
            Some(plan.id),
            Some(&recipe),
            (
                &input.target_segment.source_shot_id,
                input.target_segment.segment_index_in_shot,
            ),
        )?;
        let config = host_port.resolve_video_config(input.config_id).await?;
        let source_media_id = segment_plan_prompt_id;
        let image_uploads = validated_reference_media_uploads(
            &reference_media.image_rows,
            &input.image_uploads,
            "image",
        )
        .map_err(AppError::InvalidArgument)?;
        let video_uploads = validated_reference_media_uploads(
            &reference_media.video_rows,
            &input.video_uploads,
            "video",
        )
        .map_err(AppError::InvalidArgument)?;
        let audio_uploads = validated_reference_media_uploads(
            &reference_media.audio_rows,
            &input.audio_uploads,
            "audio",
        )
        .map_err(AppError::InvalidArgument)?;
        let redacted_local_uploads =
            image_uploads.len() + video_uploads.len() + audio_uploads.len();
        let redacted_urls = |kind: &str, uploads: &[ValidatedReferenceMediaUpload]| {
            uploads
                .iter()
                .enumerate()
                .map(|(index, upload)| {
                    format!(
                        "https://example.invalid/a4n-media/preview/reference-media-{kind}-{}/{}",
                        index + 1,
                        upload.sha256
                    )
                })
                .collect::<Vec<_>>()
        };
        let image_files = resolve_reference_media_urls(
            &reference_media.image_rows,
            redacted_urls("image", &image_uploads),
            "image_files",
        )
        .map_err(AppError::InvalidArgument)?;
        let video_files = resolve_reference_media_urls(
            &reference_media.video_rows,
            redacted_urls("video", &video_uploads),
            "video_files",
        )
        .map_err(AppError::InvalidArgument)?;
        let audio_files = resolve_reference_media_urls(
            &reference_media.audio_rows,
            redacted_urls("audio", &audio_uploads),
            "audio_files",
        )
        .map_err(AppError::InvalidArgument)?;
        let mut request = build_reference_media_video_request(
            &config,
            ResolvedReferenceMediaVideoInput {
                prompt: reference_media_prompt_from_recipe(reference_media),
                image_files,
                video_files,
                audio_files,
                duration_secs: recipe.duration_secs,
                aspect_ratio: recipe.aspect_ratio.clone(),
            },
        )
        .map_err(AppError::InvalidArgument)?;
        let mut staging_artifacts = Vec::new();
        if mode == VideoProviderValidationMode::Generate {
            self.ensure_video_capacity(config.id, config.max_concurrent)
                .await?;
        }
        if mode == VideoProviderValidationMode::Generate && redacted_local_uploads > 0 {
            let mut uploaded_image_urls = Vec::with_capacity(image_uploads.len());
            let mut uploaded_video_urls = Vec::with_capacity(video_uploads.len());
            let mut uploaded_audio_urls = Vec::with_capacity(audio_uploads.len());
            for (kind, uploads, urls) in [
                ("image", image_uploads.as_slice(), &mut uploaded_image_urls),
                ("video", video_uploads.as_slice(), &mut uploaded_video_urls),
                ("audio", audio_uploads.as_slice(), &mut uploaded_audio_urls),
            ] {
                let source_media_kind = format!("reference_media_{kind}");
                for upload in uploads {
                    let (url, artifact) = host_port
                        .upload_reference_media_input(
                            user_id,
                            project_id,
                            source_media_id,
                            &source_media_kind,
                            &upload.file_name,
                            &upload.mime_type,
                            &upload.bytes,
                        )
                        .await?;
                    urls.push(url);
                    staging_artifacts.push(artifact);
                }
            }
            request.media = a4n_video::VideoMediaInput::References {
                images: resolve_reference_media_urls(
                    &reference_media.image_rows,
                    uploaded_image_urls,
                    "image_files",
                )
                .map_err(AppError::InvalidArgument)?,
                videos: resolve_reference_media_urls(
                    &reference_media.video_rows,
                    uploaded_video_urls,
                    "video_files",
                )
                .map_err(AppError::InvalidArgument)?,
                audio: resolve_reference_media_urls(
                    &reference_media.audio_rows,
                    uploaded_audio_urls,
                    "audio_files",
                )
                .map_err(AppError::InvalidArgument)?,
            };
        }
        Ok(PreparedReferenceMediaVideoGeneration {
            project_id,
            chapter_id,
            segment_plan_prompt_id,
            shot_number,
            video_execution_identity,
            config,
            request,
            redacted_local_uploads,
            staging_artifacts,
        })
    }
}
