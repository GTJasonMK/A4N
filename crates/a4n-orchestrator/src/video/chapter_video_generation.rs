use a4n_core::{
    error::AppError,
    models::{
        ai::{generated_video, video_gen_config},
        chapter::chapter_video_prompt,
    },
    source_spans::{metadata_source_content_sha256, source_text_hash},
};
use a4n_db::collaboration::ProjectAccessRole;
use a4n_service_protocol::GenerationArtifactRef;
use a4n_video::{VideoGenerationRequest, VideoGenerationResult};
use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::OrchestratorContext;

use super::chapter_video_request_rules::{
    BoundaryFrameImageIds, ChapterVideoProviderRequestContext, ProviderPromptImageBinding,
    append_preview_duration_note, append_provider_input_note, append_provider_input_notes,
    append_video_request_prompt_length_context, apply_chapter_video_prompt_preview_override,
    attach_chapter_video_snapshot_identity,
    build_video_generation_provider_preview_prompt_with_renderer,
    build_video_generation_provider_prompt_with_renderer, chapter_video_provider_descriptor,
    enforce_cliproxy_video_media_contract, enforce_provider_image_binding_contract,
    enforce_provider_prompt_length_contract, ensure_chapter_video_prompt_preview_metadata,
    ensure_chapter_video_prompt_preview_texts, provider_specific_negative_prompt,
    sanitized_chapter_video_request_for_snapshot, video_request_prompt_length_context,
};
use super::{
    CLIPROXY_OPENAI_VIDEO_BACKEND, CliproxyGrokReferenceMode, NEWAPI_VIDEO_BACKEND,
    ProviderGenerationWorkflow, ProviderImageInputTransport, VIDEO_GENERATION_PROGRESS_TOTAL,
    VideoGenerationProgress, VideoGenerationRecordInput, VideoProviderExecutionInput,
    VideoProviderHostPort, VideoProviderRequestBody, VideoProviderRequestInput,
    VideoProviderValidationMode, boundary_frame_numbers_for_prompt,
    cliproxy_grok_model_is_image_to_video_only, ensure_newapi_first_frame_image_supported,
    first_frame_prompt_matches_video_prompt, frame_prompt_matches_video_prompt,
    image_generation_prompt_id, image_generation_source, latest_boundary_frame_image_id,
    latest_storyboard_image_id, linked_video_preview_config_id,
    previous_video_prompt_matches_current, prompt_provider_workflow,
    reference_ids_with_prompt_frame_images, reference_ids_with_storyboard_image,
    reference_image_ids, required_prompt_source_prompt_type, required_video_execution_identity,
    storyboard_prompt_matches_video_prompt, tail_frame_image_matches_prompt,
    validate_video_duration_for_config_secs, video_generation_capabilities,
};

#[derive(Debug, Clone, Deserialize)]
pub struct GenerateChapterVideoInput {
    pub user_id: i64,
    pub project_id: i64,
    pub chapter_id: i64,
    pub prompt_id: i64,
    #[serde(default)]
    pub config_id: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GenerateChapterVideosBatchInput {
    pub user_id: i64,
    pub project_id: i64,
    pub chapter_id: i64,
    pub prompt_ids: Vec<i64>,
    #[serde(default)]
    pub config_id: Option<i64>,
}

pub struct ChapterVideoRequestPreview {
    pub request: Value,
    pub request_kind: &'static str,
    pub request_title: &'static str,
    pub note: String,
}

#[derive(Debug, Clone)]
pub struct ChapterVideoImageInput {
    pub user_id: i64,
    pub project_id: i64,
    pub chapter_id: i64,
    pub mode: VideoProviderValidationMode,
    pub transport: ProviderImageInputTransport,
    pub reference_image_ids: Vec<i64>,
    pub first_frame_image_id: Option<i64>,
    pub last_frame_image_id: Option<i64>,
}

#[derive(Debug, Clone, Default)]
pub struct ResolvedChapterVideoImageInputs {
    pub reference_image_urls: Vec<String>,
    pub first_frame_image_url: Option<String>,
    pub last_frame_image_url: Option<String>,
    pub notes: Vec<String>,
    pub staging_artifacts: Vec<GenerationArtifactRef>,
}

#[async_trait]
pub trait ChapterVideoGenerationHostPort: Send + Sync {
    async fn resolve_video_config(
        &self,
        config_id: Option<i64>,
    ) -> Result<video_gen_config::Model, AppError>;

    fn resolve_image_transport(
        &self,
        backend: &str,
    ) -> Result<ProviderImageInputTransport, AppError>;

    async fn resolve_chapter_video_image_inputs(
        &self,
        input: ChapterVideoImageInput,
    ) -> Result<ResolvedChapterVideoImageInputs, AppError>;

    async fn chapter_video_provider_request_body(
        &self,
        input: VideoProviderRequestInput,
    ) -> Result<VideoProviderRequestBody, AppError>;

    async fn submit_chapter_video(
        &self,
        input: VideoProviderRequestInput,
        progress: &mut VideoGenerationProgress<'_>,
    ) -> Result<VideoGenerationResult, AppError>;

    fn runtime_config_snapshot(&self) -> Option<Value>;

    fn video_provider_host_port(&self) -> &dyn VideoProviderHostPort;
}

struct PreparedChapterVideoGeneration {
    project_id: i64,
    chapter_id: i64,
    context: ChapterVideoProviderRequestContext,
    staging_artifacts: Vec<GenerationArtifactRef>,
}

struct ChapterProviderReferences {
    boundary_frame_ids: BoundaryFrameImageIds,
    storyboard_image_id: Option<i64>,
    provider_reference_image_ids_json: Option<Value>,
    provider_reference_image_ids: Vec<i64>,
    provider_reference_image_count: usize,
}

impl OrchestratorContext {
    pub async fn preview_chapter_video_request_validated(
        &self,
        host_port: &dyn ChapterVideoGenerationHostPort,
        input: GenerateChapterVideoInput,
        prompt_override: Option<super::PreviewChapterVideoPromptOverrideInput>,
    ) -> Result<ChapterVideoRequestPreview, AppError> {
        let prepared = self
            .prepare_chapter_video_generation(
                host_port,
                input,
                VideoProviderValidationMode::Preview,
                prompt_override,
            )
            .await?;
        let body = host_port
            .chapter_video_provider_request_body(VideoProviderRequestInput {
                project_id: prepared.project_id,
                config: prepared.context.config.clone(),
                request: prepared.context.request.clone(),
                provider_idempotency_key_sha256: None,
            })
            .await?;
        let descriptor = chapter_video_provider_descriptor(&prepared.context, &body.source_label)
            .map_err(AppError::InvalidArgument)?;
        Ok(ChapterVideoRequestPreview {
            request: body.request,
            request_kind: descriptor.request_kind,
            request_title: descriptor.request_title,
            note: descriptor.note,
        })
    }

    pub async fn run_chapter_video_generation_validated(
        &self,
        host_port: &dyn ChapterVideoGenerationHostPort,
        input: GenerateChapterVideoInput,
        progress: &mut VideoGenerationProgress<'_>,
    ) -> Result<generated_video::Model, AppError> {
        progress
            .progress(
                0,
                VIDEO_GENERATION_PROGRESS_TOTAL,
                "正在检查视频提示词和配置",
            )
            .map_err(AppError::InvalidArgument)?;
        progress
            .check_cancelled("视频生成已停止")
            .map_err(AppError::InvalidArgument)?;
        let prepared = self
            .prepare_chapter_video_generation(
                host_port,
                input,
                VideoProviderValidationMode::Generate,
                None,
            )
            .await?;
        let context = &prepared.context;
        self.ensure_video_capacity(context.config.id, context.config.max_concurrent)
            .await?;

        progress
            .progress(2, VIDEO_GENERATION_PROGRESS_TOTAL, "正在创建视频生成记录")
            .map_err(AppError::InvalidArgument)?;
        progress
            .check_cancelled("视频生成已停止")
            .map_err(AppError::InvalidArgument)?;
        let video = self
            .create_chapter_video_generation_record(host_port, &prepared)
            .await?;
        if let Err(error) = self
            .ensure_created_video_capacity(
                context.config.id,
                context.config.max_concurrent,
                video.id,
            )
            .await
        {
            return self
                .mark_generated_video_failed_validated(
                    video.id,
                    Some(context.prompt.id),
                    &error.to_string(),
                )
                .await;
        }

        progress
            .progress(3, VIDEO_GENERATION_PROGRESS_TOTAL, "正在调用视频模型")
            .map_err(AppError::InvalidArgument)?;
        if let Err(message) = progress.check_cancelled("视频生成已停止") {
            return self
                .mark_generated_video_failed_validated(video.id, Some(context.prompt.id), &message)
                .await;
        }
        let prompt_length_context = video_request_prompt_length_context(&context.request);
        let mut provider_input = VideoProviderRequestInput {
            project_id: prepared.project_id,
            config: context.config.clone(),
            request: context.request.clone(),
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
            .submit_chapter_video(provider_input, progress)
            .await
            .map_err(|error| {
                append_video_request_prompt_length_context(
                    error.to_string(),
                    &prompt_length_context,
                )
            });
        if let Ok(result) = &result {
            if let Some(job_id) = result
                .provider_job_id
                .as_deref()
                .filter(|id| !id.trim().is_empty())
            {
                progress
                    .checkpoint_provider_job(video.id, &context.config, job_id)
                    .await?;
            }
        }
        if result.as_ref().is_ok_and(|result| result.bytes.is_empty()) {
            progress
                .progress(
                    4,
                    VIDEO_GENERATION_PROGRESS_TOTAL,
                    "视频模型已提交异步任务，开始等待生成结果",
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
                    config: context.config.clone(),
                    prompt_id: Some(context.prompt.id),
                    missing_result_message: "视频 provider 未返回文件或任务 ID".to_string(),
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

    async fn prepare_chapter_video_generation(
        &self,
        host_port: &dyn ChapterVideoGenerationHostPort,
        input: GenerateChapterVideoInput,
        mode: VideoProviderValidationMode,
        prompt_override: Option<super::PreviewChapterVideoPromptOverrideInput>,
    ) -> Result<PreparedChapterVideoGeneration, AppError> {
        let GenerateChapterVideoInput {
            user_id,
            project_id,
            chapter_id,
            prompt_id,
            config_id,
        } = input;
        self.verify_project_permission(project_id, user_id, ProjectAccessRole::Editor)
            .await?;
        let mut prompt = self.chapter_video_prompt(prompt_id).await?;
        if prompt.project_id != project_id || prompt.chapter_id != chapter_id {
            return Err(invalid_argument("视频提示词不属于该章节"));
        }
        let (operation, writable) = match mode {
            VideoProviderValidationMode::Generate => ("生成视频", true),
            VideoProviderValidationMode::Preview => ("预览视频生成请求", false),
        };
        let source = self
            .project_chapter_video_source(project_id, chapter_id, operation, writable)
            .await?;
        if prompt.prompt_type != "video_prompt" {
            return Err(invalid_argument("请选择 video_prompt 类型的提示词生成视频"));
        }

        let mut provider_input_note = None;
        if mode == VideoProviderValidationMode::Preview {
            apply_chapter_video_prompt_preview_override(
                &mut prompt,
                prompt_override,
                &mut provider_input_note,
            );
            ensure_chapter_video_prompt_preview_texts(&mut prompt, &mut provider_input_note);
            ensure_chapter_video_prompt_preview_metadata(&mut prompt, &mut provider_input_note)
                .map_err(AppError::InvalidArgument)?;
        } else {
            if prompt.status == "running" {
                return Err(invalid_argument(
                    "当前视频提示词正在生成视频，请先等待完成后再重新生成",
                ));
            }
            if prompt
                .metadata_json
                .as_ref()
                .and_then(|metadata| metadata.get("continuity_stale"))
                .and_then(Value::as_bool)
                == Some(true)
            {
                return Err(invalid_argument(
                    "当前首帧续接来源已被新的片段候选替换，请重新选择续接来源后再生成",
                ));
            }
            if let Some(source_hash) = metadata_source_content_sha256(prompt.metadata_json.as_ref())
            {
                if source_hash != source_text_hash(&source.source.content) {
                    return Err(invalid_argument(
                        "当前提示词基于旧正文内容，请重新生成视频提示词",
                    ));
                }
            }
        }

        let config_id = if mode == VideoProviderValidationMode::Preview {
            self.chapter_video_preview_config_id(&prompt, config_id)
                .await?
        } else {
            config_id
        };
        let config = host_port.resolve_video_config(config_id).await?;
        if mode == VideoProviderValidationMode::Generate {
            validate_video_duration_for_config_secs(&config, prompt.duration_secs)
                .map_err(AppError::InvalidArgument)?;
        } else {
            append_preview_duration_note(&mut provider_input_note, &config, prompt.duration_secs);
        }

        let prompt_reference_image_ids_json = prompt.reference_image_ids_json.clone();
        let prompt_reference_image_ids =
            reference_image_ids(prompt_reference_image_ids_json.as_ref())
                .map_err(AppError::InvalidArgument)?;
        let prompt_reference_image_count = prompt_reference_image_ids.len();
        let source_prompt_type = Some(
            required_prompt_source_prompt_type(prompt.id, prompt.metadata_json.as_ref())
                .map_err(AppError::InvalidArgument)?
                .to_string(),
        );
        let video_execution_identity =
            required_video_execution_identity(prompt.id, prompt.metadata_json.as_ref())
                .map_err(AppError::InvalidArgument)?;
        let provider_workflow =
            prompt_provider_workflow(&prompt).map_err(AppError::InvalidArgument)?;
        let prompt_workflow_mode = prompt.workflow_mode.clone();
        if provider_workflow.uses_text_only() && !prompt_reference_image_ids.is_empty() {
            return Err(invalid_argument(
                "纯文本生成不能绑定参考图片；请清空当前片段参考图后重试",
            ));
        }
        if provider_workflow.uses_text_only()
            && config.backend == CLIPROXY_OPENAI_VIDEO_BACKEND
            && cliproxy_grok_model_is_image_to_video_only(&config.model)
        {
            return Err(invalid_argument(format!(
                "当前视频模型 {} 仅支持图生视频，不能使用纯文本生成",
                config.model
            )));
        }
        if provider_workflow.uses_keyframes() {
            let capabilities = video_generation_capabilities(
                &config.backend,
                &config.model,
                config.provider_options_json.as_ref(),
            );
            let keyframe_count = prompt_reference_image_ids.len();
            if !capabilities.supports_keyframes
                || !(capabilities.keyframe_min as usize..=capabilities.keyframe_max as usize)
                    .contains(&keyframe_count)
            {
                return Err(invalid_argument(format!(
                    "当前视频服务关键帧数量范围为 {}-{} 张，当前 {} 张",
                    capabilities.keyframe_min, capabilities.keyframe_max, keyframe_count
                )));
            }
        }
        let storyboard_workflow = provider_workflow.uses_storyboard();
        let cliproxy_grok_mode =
            super::parse_cliproxy_grok_reference_mode(config.provider_options_json.as_ref())
                .map_err(AppError::InvalidArgument)?;
        let cliproxy_first_frame_mode = config.backend == CLIPROXY_OPENAI_VIDEO_BACKEND
            && cliproxy_grok_mode == CliproxyGrokReferenceMode::FirstFrame;
        let cliproxy_reference_mode = config.backend == CLIPROXY_OPENAI_VIDEO_BACKEND
            && cliproxy_grok_mode == CliproxyGrokReferenceMode::Reference;
        enforce_provider_image_binding_contract(
            &config.backend,
            provider_workflow,
            cliproxy_grok_mode,
            prompt_reference_image_count,
            mode,
            &mut provider_input_note,
        )
        .map_err(AppError::InvalidArgument)?;

        let transport = host_port.resolve_image_transport(&config.backend)?;
        if transport == ProviderImageInputTransport::RemoteTempUpload {
            append_provider_input_note(
                &mut provider_input_note,
                "远端临时媒体上传：生成时会携带用户、项目和来源图片归属上传，并要求远端返回最长 24 小时有效的 HTTPS artifact；预览省略真实图片数据。",
            );
        }
        let references = self
            .resolve_chapter_provider_references(
                project_id,
                chapter_id,
                &prompt,
                &prompt_reference_image_ids,
                prompt_reference_image_ids_json.as_ref(),
                provider_workflow,
                cliproxy_reference_mode,
                mode,
                &mut provider_input_note,
            )
            .await?;
        let cliproxy_has_first_frame = config.backend == CLIPROXY_OPENAI_VIDEO_BACKEND
            && references.boundary_frame_ids.first.is_some();
        let should_send_reference_images = config.backend != NEWAPI_VIDEO_BACKEND
            && !(cliproxy_first_frame_mode && (cliproxy_has_first_frame || storyboard_workflow));
        let selected_reference_image_ids = if should_send_reference_images {
            references.provider_reference_image_ids.clone()
        } else if cliproxy_first_frame_mode && !cliproxy_has_first_frame && !storyboard_workflow {
            prompt_reference_image_ids.iter().copied().take(1).collect()
        } else {
            Vec::new()
        };
        let selected_first_frame_image_id = if cliproxy_reference_mode || storyboard_workflow {
            None
        } else {
            references.boundary_frame_ids.first
        };
        let selected_last_frame_image_id = if storyboard_workflow
            || config.backend == NEWAPI_VIDEO_BACKEND
            || config.backend == CLIPROXY_OPENAI_VIDEO_BACKEND
        {
            None
        } else {
            references.boundary_frame_ids.last
        };
        let image_inputs = host_port
            .resolve_chapter_video_image_inputs(ChapterVideoImageInput {
                user_id,
                project_id,
                chapter_id,
                mode,
                transport,
                reference_image_ids: selected_reference_image_ids,
                first_frame_image_id: selected_first_frame_image_id,
                last_frame_image_id: selected_last_frame_image_id,
            })
            .await?;
        append_provider_input_notes(&mut provider_input_note, image_inputs.notes);
        if cliproxy_first_frame_mode && !storyboard_workflow && prompt_reference_image_count > 0 {
            let note = if references.boundary_frame_ids.first.is_some() {
                format!(
                    "CLIProxy/Grok 首帧模式已使用生成首帧作为 input_reference，{} 张提示词参考图不会作为 reference_images 发送。",
                    prompt_reference_image_count
                )
            } else {
                format!(
                    "CLIProxy/Grok 首帧模式会把第一张提示词参考图作为 input_reference 使用；其余 {} 张提示词参考图不会发送。",
                    prompt_reference_image_count - 1
                )
            };
            append_provider_input_note(&mut provider_input_note, note);
        }

        if config.backend == NEWAPI_VIDEO_BACKEND && mode == VideoProviderValidationMode::Generate {
            ensure_newapi_first_frame_image_supported(
                image_inputs.first_frame_image_url.as_deref(),
            )
            .map_err(AppError::InvalidArgument)?;
        }
        let first_frame_image_url_available = image_inputs.first_frame_image_url.is_some()
            || (cliproxy_first_frame_mode
                && !storyboard_workflow
                && !image_inputs.reference_image_urls.is_empty());
        let last_frame_image_url_available = image_inputs.last_frame_image_url.is_some();
        enforce_cliproxy_video_media_contract(
            &config.backend,
            &config.model,
            cliproxy_grok_mode,
            references.provider_reference_image_count,
            first_frame_image_url_available,
            prompt.duration_secs,
            mode,
            &mut provider_input_note,
        )
        .map_err(AppError::InvalidArgument)?;
        let image_binding = ProviderPromptImageBinding {
            has_first_frame_image: first_frame_image_url_available,
            has_last_frame_image: last_frame_image_url_available,
            reference_image_count: image_inputs.reference_image_urls.len(),
            has_storyboard_reference_image: storyboard_workflow
                && references.storyboard_image_id.is_some(),
        };
        let full_prompt = match mode {
            VideoProviderValidationMode::Generate => {
                build_video_generation_provider_prompt_with_renderer(self, &prompt, image_binding)
                    .await
            }
            VideoProviderValidationMode::Preview => {
                build_video_generation_provider_preview_prompt_with_renderer(
                    self,
                    &prompt,
                    image_binding,
                )
                .await
            }
        }
        .map_err(AppError::InvalidArgument)?;
        let prompt_source = prompt
            .metadata_json
            .as_ref()
            .and_then(|metadata| metadata.get("source"))
            .cloned();
        let request = VideoGenerationRequest {
            prompt: full_prompt,
            negative_prompt: provider_specific_negative_prompt(prompt.negative_prompt.as_deref()),
            width: config.default_width,
            height: config.default_height,
            duration_secs: prompt.duration_secs,
            fps: config.default_fps,
            model: config.model.clone(),
            first_frame_image_url: image_inputs.first_frame_image_url,
            last_frame_image_url: image_inputs.last_frame_image_url,
            reference_image_urls: image_inputs.reference_image_urls,
            reference_media: None,
        };
        enforce_provider_prompt_length_contract(
            &config.backend,
            &request.prompt,
            mode,
            &mut provider_input_note,
        )
        .map_err(AppError::InvalidArgument)?;
        Ok(PreparedChapterVideoGeneration {
            project_id,
            chapter_id,
            staging_artifacts: image_inputs.staging_artifacts,
            context: ChapterVideoProviderRequestContext {
                prompt,
                config,
                request,
                prompt_reference_image_ids: prompt_reference_image_ids_json,
                provider_reference_image_ids: references.provider_reference_image_ids_json,
                prompt_source,
                workflow_mode: prompt_workflow_mode,
                source_prompt_type,
                video_execution_identity,
                boundary_frame_ids: references.boundary_frame_ids,
                prompt_reference_image_count,
                provider_reference_image_count: references.provider_reference_image_count,
                first_frame_image_url_available,
                last_frame_image_url_available,
                provider_input_note,
            },
        })
    }

    async fn create_chapter_video_generation_record(
        &self,
        host_port: &dyn ChapterVideoGenerationHostPort,
        prepared: &PreparedChapterVideoGeneration,
    ) -> Result<generated_video::Model, AppError> {
        let context = &prepared.context;
        let snapshot_request = sanitized_chapter_video_request_for_snapshot(context)
            .map_err(AppError::InvalidArgument)?;
        let body = host_port
            .chapter_video_provider_request_body(VideoProviderRequestInput {
                project_id: prepared.project_id,
                config: context.config.clone(),
                request: snapshot_request,
                provider_idempotency_key_sha256: None,
            })
            .await?;
        let descriptor =
            chapter_video_provider_descriptor(context, &format!("生成时{}", body.source_label))
                .map_err(AppError::InvalidArgument)?;
        let provider_request_snapshot = attach_chapter_video_snapshot_identity(
            super::provider_request_snapshot_value(
                "chapter_video",
                descriptor.request_title,
                descriptor.request_kind,
                Some("chapter_video_prompt"),
                Some(descriptor.note),
                body.request,
            ),
            context,
        );
        let metadata = json!({
            "config_id": context.config.id,
            "prompt_id": context.prompt.id,
            "shot_number": context.prompt.shot_number,
            "source": context.prompt_source.clone(),
            "source_prompt_type": context.source_prompt_type.clone(),
            "video_execution_identity": context.video_execution_identity.clone(),
            "generation_asset_fingerprint": context.video_execution_identity.get("generation_asset_fingerprint"),
            "prompt_output_language": context.video_execution_identity.get("prompt_output_language"),
            "prompt_reference_image_ids": context.prompt_reference_image_ids.clone(),
            "prompt_reference_image_count": context.prompt_reference_image_count,
            "provider_reference_image_ids": context.provider_reference_image_ids.clone(),
            "provider_reference_image_count": context.provider_reference_image_count,
            "start_frame_image_id": context.boundary_frame_ids.first,
            "boundary_frame_image_ids": {
                "first": context.boundary_frame_ids.first,
                "last": context.boundary_frame_ids.last
            },
            "boundary_frame_image_count": usize::from(context.boundary_frame_ids.first.is_some())
                + usize::from(context.boundary_frame_ids.last.is_some()),
            "boundary_frame_image_url_count": usize::from(context.first_frame_image_url_available)
                + usize::from(context.last_frame_image_url_available),
            "boundary_frame_request": {
                "first_frame_image_url": context.first_frame_image_url_available,
                "last_frame_image_url": context.last_frame_image_url_available,
            },
            "provider_request_snapshot": provider_request_snapshot.clone(),
            "runtime_config_snapshot": host_port.runtime_config_snapshot(),
        });
        self.create_video_generation_record_validated(VideoGenerationRecordInput {
            project_id: prepared.project_id,
            chapter_id: Some(prepared.chapter_id),
            video_type: "chapter_video".into(),
            workflow_mode: Some(context.workflow_mode.clone()),
            prompt: context.request.prompt.clone(),
            negative_prompt: context.request.negative_prompt.clone(),
            model: context.config.model.clone(),
            width: context.request.width,
            height: context.request.height,
            duration_secs: context.request.duration_secs as f32,
            fps: context.request.fps,
            source_video_id: None,
            source_image_id: context.boundary_frame_ids.first,
            metadata_json: Some(metadata),
            provider_request_snapshot,
            provider_backend: context.config.backend.clone(),
            step_key: "generate_chapter_video".to_string(),
            title: "章节视频 provider 生成".to_string(),
            prompt_template_name: Some("chapter_video_prompt".to_string()),
            prompt_id: Some(context.prompt.id),
        })
        .await
    }

    async fn chapter_video_preview_config_id(
        &self,
        prompt: &chapter_video_prompt::Model,
        requested_config_id: Option<i64>,
    ) -> Result<Option<i64>, AppError> {
        if requested_config_id.is_some() {
            return Ok(requested_config_id);
        }
        let Some(video_id) = prompt.generated_video_id else {
            return Ok(None);
        };
        let video = self.generated_video_by_id(video_id).await?.ok_or_else(|| {
            AppError::NotFound(format!(
                "视频提示词 #{} 关联的视频 #{video_id} 不存在",
                prompt.id
            ))
        })?;
        linked_video_preview_config_id(
            prompt.id,
            prompt.project_id,
            prompt.chapter_id,
            video.id,
            video.project_id,
            video.chapter_id,
            video.metadata_json.as_ref(),
        )
        .map_err(AppError::InvalidArgument)
    }

    pub(crate) async fn validate_chapter_video_batch_prompts(
        &self,
        input: &GenerateChapterVideosBatchInput,
    ) -> Result<Vec<i64>, AppError> {
        if input.prompt_ids.is_empty() {
            return Err(invalid_argument("批量生成视频至少需要选择一条视频提示词"));
        }
        self.verify_project_permission(input.project_id, input.user_id, ProjectAccessRole::Editor)
            .await?;
        self.project_chapter_video_source(input.project_id, input.chapter_id, "批量生成视频", true)
            .await?;
        let mut unique_prompt_ids = input.prompt_ids.clone();
        unique_prompt_ids.sort_unstable();
        unique_prompt_ids.dedup();
        if unique_prompt_ids.len() != input.prompt_ids.len() {
            return Err(invalid_argument("批量生成视频 prompt_ids 不能重复"));
        }
        let rows = self
            .chapter_video_prompts_for_batch_generation(&unique_prompt_ids)
            .await?;
        if rows.len() != unique_prompt_ids.len() {
            return Err(invalid_argument("部分视频提示词不存在，不能批量生成视频"));
        }
        for row in &rows {
            if row.project_id != input.project_id || row.chapter_id != input.chapter_id {
                return Err(invalid_argument(format!(
                    "视频提示词 #{} 不属于当前项目章节",
                    row.id
                )));
            }
            if row.prompt_type != "video_prompt" {
                return Err(invalid_argument(format!(
                    "视频提示词 #{} 不是 video_prompt 类型",
                    row.id
                )));
            }
            super::ensure_provider_generation_workflow(row).map_err(AppError::InvalidArgument)?;
            if row.status == "running" {
                return Err(invalid_argument(format!(
                    "视频提示词 #{} 正在运行，不能批量生成视频",
                    row.id
                )));
            }
            if row
                .metadata_json
                .as_ref()
                .and_then(|metadata| metadata.get("continuity_stale"))
                .and_then(Value::as_bool)
                == Some(true)
            {
                return Err(invalid_argument(format!(
                    "视频提示词 #{} 的首帧续接来源已失效，请重新选择后再生成",
                    row.id
                )));
            }
        }
        Ok(rows.into_iter().map(|row| row.id).collect())
    }

    #[allow(clippy::too_many_arguments)]
    async fn resolve_chapter_provider_references(
        &self,
        project_id: i64,
        chapter_id: i64,
        prompt: &chapter_video_prompt::Model,
        prompt_reference_image_ids: &[i64],
        prompt_reference_image_ids_json: Option<&Value>,
        provider_workflow: ProviderGenerationWorkflow,
        cliproxy_reference_mode: bool,
        mode: VideoProviderValidationMode,
        provider_input_note: &mut Option<String>,
    ) -> Result<ChapterProviderReferences, AppError> {
        let storyboard_workflow = provider_workflow.uses_storyboard();
        let (boundary_frame_ids, storyboard_image_id) =
            if provider_workflow.uses_text_only() || provider_workflow.uses_keyframes() {
                (BoundaryFrameImageIds::default(), None)
            } else if provider_workflow.uses_first_frame_chain() {
                let start = self
                    .first_frame_chain_start_image_id_for_prompt(project_id, chapter_id, prompt)
                    .await?;
                if start.is_none() {
                    let message = super::first_frame_chain_missing_start_image_message(prompt);
                    missing_image_error_or_note(
                        mode,
                        provider_input_note,
                        message,
                        "当前首帧串联起始帧缺失",
                    )?;
                }
                (
                    BoundaryFrameImageIds {
                        first: start,
                        last: None,
                    },
                    None,
                )
            } else if storyboard_workflow {
                let storyboard_image_id = self
                    .storyboard_image_id_for_prompt(project_id, chapter_id, prompt)
                    .await?;
                if storyboard_image_id.is_none() {
                    missing_image_error_or_note(
                        mode,
                        provider_input_note,
                        super::storyboard_missing_image_message(prompt),
                        "当前分镜图缺失",
                    )?;
                }
                (BoundaryFrameImageIds::default(), storyboard_image_id)
            } else {
                let frame_ids = self
                    .boundary_frame_image_ids_for_prompt(project_id, chapter_id, prompt)
                    .await?;
                if let Some(message) = super::boundary_frame_missing_image_message(
                    prompt,
                    frame_ids.first.is_some(),
                    frame_ids.last.is_some(),
                ) {
                    missing_image_error_or_note(
                        mode,
                        provider_input_note,
                        message,
                        "当前首尾帧图片不完整",
                    )?;
                }
                (frame_ids, None)
            };
        let provider_reference_image_ids = if provider_workflow.uses_text_only() {
            Vec::new()
        } else if provider_workflow.uses_keyframes() {
            prompt_reference_image_ids.to_vec()
        } else if storyboard_workflow {
            reference_ids_with_storyboard_image(storyboard_image_id, prompt_reference_image_ids)
        } else if cliproxy_reference_mode {
            reference_ids_with_prompt_frame_images(
                boundary_frame_ids.first,
                boundary_frame_ids.last,
                prompt_reference_image_ids,
            )
        } else {
            prompt_reference_image_ids.to_vec()
        };
        let provider_reference_ids_are_authoritative = provider_workflow.uses_text_only()
            || provider_workflow.uses_keyframes()
            || storyboard_workflow
            || cliproxy_reference_mode;
        if cliproxy_reference_mode
            && (boundary_frame_ids.first.is_some() || boundary_frame_ids.last.is_some())
        {
            append_provider_input_note(
                provider_input_note,
                format!(
                    "CLIProxy/Grok 参考模式已将边界帧图片作为普通 reference_images 发送；实际参考图 ID 顺序：{}",
                    joined_ids(&provider_reference_image_ids)
                ),
            );
        }
        if storyboard_workflow && storyboard_image_id.is_some() {
            append_provider_input_note(
                provider_input_note,
                format!(
                    "分镜模式已将分镜图作为第 1 张 reference_images 发送，后续为素材参考图；实际参考图 ID 顺序：{}",
                    joined_ids(&provider_reference_image_ids)
                ),
            );
        }
        let provider_reference_image_ids_json = if provider_reference_ids_are_authoritative {
            Some(json!(provider_reference_image_ids.clone()))
        } else {
            prompt_reference_image_ids_json.cloned()
        };
        Ok(ChapterProviderReferences {
            boundary_frame_ids,
            storyboard_image_id,
            provider_reference_image_ids_json,
            provider_reference_image_count: provider_reference_image_ids.len(),
            provider_reference_image_ids,
        })
    }

    async fn boundary_frame_image_ids_for_prompt(
        &self,
        project_id: i64,
        chapter_id: i64,
        prompt: &chapter_video_prompt::Model,
    ) -> Result<BoundaryFrameImageIds, AppError> {
        let (first_frame_number, last_frame_number) =
            boundary_frame_numbers_for_prompt(prompt).map_err(AppError::InvalidArgument)?;
        let frame_prompts = self
            .chapter_video_boundary_frame_prompt_candidates(
                project_id,
                chapter_id,
                first_frame_number,
                last_frame_number,
            )
            .await?;
        let first_prompt_id = frame_prompts
            .iter()
            .filter(|frame| frame_prompt_matches_video_prompt(prompt, frame))
            .find(|frame| frame.shot_number == first_frame_number)
            .map(|frame| frame.id);
        let last_prompt_id = frame_prompts
            .iter()
            .filter(|frame| frame_prompt_matches_video_prompt(prompt, frame))
            .find(|frame| frame.shot_number == last_frame_number)
            .map(|frame| frame.id);
        if first_prompt_id.is_none() && last_prompt_id.is_none() {
            return Ok(BoundaryFrameImageIds::default());
        }
        let images = self
            .generated_images_for_video_prompt_references(project_id, chapter_id)
            .await?;
        Ok(BoundaryFrameImageIds {
            first: frame_prompts
                .iter()
                .find(|frame| Some(frame.id) == first_prompt_id)
                .and_then(|frame| latest_boundary_frame_image_id(&images, frame)),
            last: frame_prompts
                .iter()
                .find(|frame| Some(frame.id) == last_prompt_id)
                .and_then(|frame| latest_boundary_frame_image_id(&images, frame)),
        })
    }

    async fn first_frame_chain_start_image_id_for_prompt(
        &self,
        project_id: i64,
        chapter_id: i64,
        prompt: &chapter_video_prompt::Model,
    ) -> Result<Option<i64>, AppError> {
        if prompt.shot_number <= 1 {
            let candidates = self
                .chapter_video_first_frame_prompt_candidates(project_id, chapter_id)
                .await?;
            let Some(first_frame_prompt) = candidates
                .iter()
                .find(|frame| first_frame_prompt_matches_video_prompt(prompt, frame))
            else {
                return Ok(None);
            };
            let images = self
                .generated_images_for_video_prompt_references(project_id, chapter_id)
                .await?;
            return Ok(images
                .iter()
                .find(|image| {
                    let metadata = image.metadata_json.as_ref();
                    image_generation_prompt_id(metadata) == Some(first_frame_prompt.id)
                        && image_generation_source(metadata) == Some("video_first_frame_prompt")
                })
                .map(|image| image.id));
        }
        if prompt
            .metadata_json
            .as_ref()
            .and_then(|metadata| metadata.get("guidance_mode"))
            .and_then(Value::as_str)
            == Some("continuity")
        {
            let source_video_id = prompt
                .metadata_json
                .as_ref()
                .and_then(|metadata| metadata.get("continuity_source"))
                .and_then(|source| source.get("video_id"))
                .and_then(Value::as_i64)
                .ok_or_else(|| {
                    invalid_argument("首帧续接提示词缺少已冻结的 continuity_source.video_id")
                })?;
            let source_video = self
                .generated_video_by_id(source_video_id)
                .await?
                .ok_or_else(|| {
                    invalid_argument(format!(
                        "连续性来源视频 #{source_video_id} 已不存在，请重新选择"
                    ))
                })?;
            if source_video.project_id != project_id
                || source_video.chapter_id != Some(chapter_id)
                || source_video.status != "completed"
            {
                return Err(invalid_argument(format!(
                    "连续性来源视频 #{source_video_id} 不再是当前章节的已完成视频，请重新选择"
                )));
            }
            let source_prompt_id = source_video
                .metadata_json
                .as_ref()
                .and_then(|metadata| metadata.get("prompt_id"))
                .and_then(Value::as_i64)
                .ok_or_else(|| {
                    invalid_argument(format!(
                        "连续性来源视频 #{source_video_id} 缺少 prompt_id，无法定位尾帧"
                    ))
                })?;
            let source_prompt = self.chapter_video_prompt(source_prompt_id).await?;
            let images = self
                .generated_images_for_video_prompt_references(project_id, chapter_id)
                .await?;
            return Ok(images
                .iter()
                .find(|image| tail_frame_image_matches_prompt(image, &source_prompt))
                .map(|image| image.id));
        }
        let previous_prompt = self
            .chapter_video_first_frame_chain_prompt_candidates(project_id, chapter_id)
            .await?
            .into_iter()
            .find(|candidate| previous_video_prompt_matches_current(prompt, candidate));
        let Some(previous_prompt) = previous_prompt else {
            return Ok(None);
        };
        let images = self
            .generated_images_for_video_prompt_references(project_id, chapter_id)
            .await?;
        Ok(images
            .iter()
            .find(|image| tail_frame_image_matches_prompt(image, &previous_prompt))
            .map(|image| image.id))
    }

    async fn storyboard_image_id_for_prompt(
        &self,
        project_id: i64,
        chapter_id: i64,
        prompt: &chapter_video_prompt::Model,
    ) -> Result<Option<i64>, AppError> {
        let prompts = self
            .chapter_video_storyboard_prompt_candidates(project_id, chapter_id, prompt.shot_number)
            .await?;
        let Some(storyboard_prompt) = prompts
            .iter()
            .find(|candidate| storyboard_prompt_matches_video_prompt(prompt, candidate))
        else {
            return Ok(None);
        };
        let images = self
            .generated_images_for_video_prompt_references(project_id, chapter_id)
            .await?;
        Ok(latest_storyboard_image_id(&images, storyboard_prompt))
    }
}

fn missing_image_error_or_note(
    mode: VideoProviderValidationMode,
    note: &mut Option<String>,
    message: String,
    label: &str,
) -> Result<(), AppError> {
    if mode == VideoProviderValidationMode::Generate {
        return Err(AppError::InvalidArgument(message));
    }
    append_provider_input_note(
        note,
        format!("{label}：{message}；此处仅展示 provider 请求结构，真实生成仍需补齐该输入"),
    );
    Ok(())
}

fn joined_ids(ids: &[i64]) -> String {
    ids.iter()
        .map(i64::to_string)
        .collect::<Vec<_>>()
        .join(", ")
}

fn invalid_argument(message: impl Into<String>) -> AppError {
    AppError::InvalidArgument(message.into())
}
