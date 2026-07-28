use a4n_core::{
    error::AppError,
    models::ai::{generated_video, video_gen_config},
};
use a4n_db::collaboration::ProjectAccessRole;
use a4n_service_protocol::GenerationArtifactRef;
use a4n_video::{VideoGenerationRequest, VideoGenerationResult};
use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::OrchestratorContext;

use super::{
    VIDEO_GENERATION_PROGRESS_TOTAL, VideoGenerationProgress, VideoGenerationRecordInput,
    VideoProviderExecutionInput, VideoProviderHostPort, VideoProviderRequestBody,
    VideoProviderRequestInput, VideoProviderValidationMode, provider_request_snapshot_value,
    reference_media_rules::{
        PreparedReferenceMediaVideoRequest, ReferenceMediaVideoInput,
        build_reference_media_video_request, reference_media_provider_request_note,
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
        upload: &a4n_api_contract::video::ReferenceMediaUploadInput,
    ) -> Result<(String, GenerationArtifactRef), AppError> {
        let _ = (
            owner_user_id,
            project_id,
            source_media_id,
            source_media_kind,
            upload,
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
    segment_prompt_id: i64,
    shot_number: i32,
    config: video_gen_config::Model,
    request: VideoGenerationRequest,
    provider_input_note: Option<String>,
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
                "预览参考媒体视频请求",
                false,
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
        let note = reference_media_provider_request_note(
            &format!("当前表单；{}", body.source_label),
            &prepared.config,
            &prepared.request,
            prepared.provider_input_note.as_deref(),
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
        progress
            .progress(1, VIDEO_GENERATION_PROGRESS_TOTAL, "正在校验参考媒体请求")
            .map_err(AppError::InvalidArgument)?;
        progress
            .check_cancelled("参考媒体视频生成已停止")
            .map_err(AppError::InvalidArgument)?;
        let prepared = self
            .prepare_reference_media_video_generation(
                host_port,
                input,
                VideoProviderValidationMode::Generate,
                "生成参考媒体视频",
                true,
            )
            .await?;
        self.ensure_video_capacity(prepared.config.id, prepared.config.max_concurrent)
            .await?;

        progress
            .progress(2, VIDEO_GENERATION_PROGRESS_TOTAL, "正在创建视频生成记录")
            .map_err(AppError::InvalidArgument)?;
        progress
            .check_cancelled("参考媒体视频生成已停止")
            .map_err(AppError::InvalidArgument)?;
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
            prepared.provider_input_note.as_deref(),
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
        let reference_media =
            prepared.request.reference_media.as_ref().ok_or_else(|| {
                AppError::InvalidArgument("参考媒体请求缺少 reference_media".into())
            })?;
        let metadata = json!({
            "prompt_id": prepared.segment_prompt_id,
            "shot_number": prepared.shot_number,
            "config_id": prepared.config.id,
            "provider_backend": prepared.config.backend,
            "reference_media": {
                "functionMode": "omni_reference",
                "image_file_count": reference_media.image_files.len(),
                "video_file_count": reference_media.video_files.len(),
                "audio_file_count": reference_media.audio_files.len(),
                "image_files": &reference_media.image_files,
                "video_files": &reference_media.video_files,
                "audio_files": &reference_media.audio_files,
            },
            "provider_request_snapshot": provider_request_snapshot,
            "runtime_config_snapshot": host_port.runtime_config_snapshot(),
        });
        let video = self
            .create_video_generation_record_validated(VideoGenerationRecordInput {
                project_id: prepared.project_id,
                chapter_id: Some(prepared.chapter_id),
                video_type: "reference_media_video".into(),
                workflow_mode: Some("reference_media".to_string()),
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
                prompt_id: Some(prepared.segment_prompt_id),
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

        progress
            .progress(3, VIDEO_GENERATION_PROGRESS_TOTAL, "正在调用 Seedance 2.0")
            .map_err(AppError::InvalidArgument)?;
        if let Err(message) = progress.check_cancelled("参考媒体视频生成已停止") {
            return self
                .mark_generated_video_failed_validated(video.id, None, &message)
                .await;
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
        if let Ok(result) = &result {
            if let Some(job_id) = result
                .provider_job_id
                .as_deref()
                .filter(|id| !id.trim().is_empty())
            {
                progress
                    .checkpoint_provider_job(video.id, &prepared.config, job_id)
                    .await?;
            }
        }
        if result.as_ref().is_ok_and(|result| result.bytes.is_empty()) {
            progress
                .progress(
                    4,
                    VIDEO_GENERATION_PROGRESS_TOTAL,
                    "Seedance 2.0 已提交异步任务，开始等待生成结果",
                )
                .map_err(AppError::InvalidArgument)?;
        } else {
            progress
                .progress(
                    6,
                    VIDEO_GENERATION_PROGRESS_TOTAL,
                    "视频模型已返回，正在保存文件",
                )
                .map_err(AppError::InvalidArgument)?;
        }
        let updated = self
            .apply_video_provider_result_validated(
                VideoProviderExecutionInput {
                    video,
                    config: prepared.config,
                    prompt_id: Some(prepared.segment_prompt_id),
                    missing_result_message: "Seedance 2.0 未返回文件或任务 ID".to_string(),
                    cancel_token: progress.cancel_token(),
                },
                result,
                host_port.video_provider_host_port(),
            )
            .await?;
        progress
            .progress(7, VIDEO_GENERATION_PROGRESS_TOTAL, "视频文件已保存")
            .map_err(AppError::InvalidArgument)?;
        Ok(updated)
    }

    async fn prepare_reference_media_video_generation(
        &self,
        host_port: &dyn ReferenceMediaVideoGenerationHostPort,
        input: GenerateReferenceMediaVideoInput,
        mode: VideoProviderValidationMode,
        operation: &str,
        writable: bool,
    ) -> Result<PreparedReferenceMediaVideoGeneration, AppError> {
        let GenerateReferenceMediaVideoInput {
            user_id,
            project_id,
            chapter_id,
            mut input,
        } = input;
        self.verify_project_permission(project_id, user_id, ProjectAccessRole::Editor)
            .await?;
        self.project_chapter_video_source(project_id, chapter_id, operation, writable)
            .await?;
        let segment_prompt_id = input.segment_prompt_id;
        let shot_number = input.shot_number;
        let prompt = self
            .chapter_video_prompt_for_import(project_id, chapter_id, segment_prompt_id)
            .await?;
        if prompt.shot_number != shot_number {
            return Err(AppError::InvalidArgument(format!(
                "参考媒体片段序号不匹配：提示词 #{} 属于片段 {}，请求为片段 {}",
                segment_prompt_id, prompt.shot_number, shot_number
            )));
        }
        let config = host_port.resolve_video_config(input.config_id).await?;
        let source_media_id = segment_prompt_id;
        let mut staging_artifacts = Vec::new();
        for (kind, uploads, urls) in [
            (
                "image",
                std::mem::take(&mut input.image_uploads),
                &mut input.image_files,
            ),
            (
                "video",
                std::mem::take(&mut input.video_uploads),
                &mut input.video_files,
            ),
            (
                "audio",
                std::mem::take(&mut input.audio_uploads),
                &mut input.audio_files,
            ),
        ] {
            for upload in uploads {
                let mime = upload.mime_type.trim().to_ascii_lowercase();
                if !mime.starts_with(&format!("{kind}/")) {
                    return Err(AppError::InvalidArgument(format!(
                        "{kind}_uploads 的 MIME 必须以 {kind}/ 开头"
                    )));
                }
                if mode == VideoProviderValidationMode::Preview {
                    urls.push(super::reference_media_preview_placeholder_url(
                        kind,
                        urls.len(),
                    ));
                    continue;
                }
                let (url, artifact) = host_port
                    .upload_reference_media_input(
                        user_id,
                        project_id,
                        source_media_id,
                        &format!("reference_media_{kind}"),
                        &upload,
                    )
                    .await?;
                urls.push(url);
                staging_artifacts.push(artifact);
            }
        }
        let PreparedReferenceMediaVideoRequest {
            request,
            provider_input_note,
        } = build_reference_media_video_request(&config, input, mode)
            .map_err(AppError::InvalidArgument)?;
        Ok(PreparedReferenceMediaVideoGeneration {
            project_id,
            chapter_id,
            segment_prompt_id,
            shot_number,
            config,
            request,
            provider_input_note,
            staging_artifacts,
        })
    }
}
