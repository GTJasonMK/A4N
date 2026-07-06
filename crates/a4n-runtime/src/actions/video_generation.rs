use self::boundary_frames::{
    BoundaryFrameImageIds, boundary_frame_image_ids_for_prompt, boundary_frame_numbers_for_prompt,
};
use self::config_state::{
    ensure_created_video_capacity, ensure_video_capacity, resolve_video_config,
};
use self::first_frame_chain::first_frame_chain_start_image_id_for_prompt;
use self::job_lifecycle::{
    has_active_provider_job as lifecycle_has_active_provider_job, normalized_provider_job_id,
    save_completed_video_result, wait_for_canvas_graph_video_job_completion,
    wait_for_video_job_completion,
};
use self::media::{
    ProviderImageInputTransport, reference_image_preview_placeholder_by_id,
    reference_image_preview_placeholders, reference_image_provider_input_by_id,
    reference_image_provider_inputs, remote_temp_upload_configured, required_video_file_extension,
};
use self::provider_request::{
    CliproxyGrokReferenceMode, append_preview_duration_note, append_provider_input_note,
    build_chapter_video_provider_request, chapter_video_provider_request_snapshot,
    cliproxy_grok_reference_mode, ensure_provider_generation_workflow,
    model_request_preview_from_provider_snapshot, provider_request_snapshot_value,
    validate_video_provider_request_snapshot_owner, video_provider_preview_payload,
};
#[cfg(test)]
use self::provider_request::{
    apply_chapter_video_prompt_preview_override, boundary_frame_missing_image_message,
    build_video_generation_provider_preview_prompt_with_binding,
    build_video_generation_provider_prompt_with_binding, enforce_cliproxy_video_media_contract,
    enforce_provider_image_binding_contract, enforce_provider_prompt_length_contract,
    ensure_chapter_video_prompt_preview_metadata, ensure_chapter_video_prompt_preview_texts,
    ensure_newapi_first_frame_image_supported, linked_video_preview_config_id,
    prompt_is_first_frame_chain, provider_default_image_input_transport_with_remote,
    provider_image_input_transport_with_override, provider_specific_negative_prompt,
    reference_ids_with_boundary_frame_images,
};
use self::storyboard_frames::storyboard_image_id_for_prompt;
use crate::AppState;
use crate::actions::access;
use crate::actions::image_generation::{
    run_queued_request_with_cancel, run_queued_request_with_cancel_and_workflow,
};
pub use crate::actions::request_preview::{ModelRequestPreview, ModelRequestPreviewStep};
use crate::actions::request_preview::{model_request_preview, provider_request_step};
use crate::actions::video_prompt_text::{
    MIN_PROVIDER_MOTION_PROMPT_CHARS, MIN_PROVIDER_VISUAL_PROMPT_CHARS,
    ProviderPromptTextViolation, provider_prompt_text_violation, validate_provider_model_text,
};
use crate::actions::video_prompts::{
    ensure_prompt_matches_current_chapter, resolve_project_chapter_video_source,
};
use crate::file_utils::{data_dir_relative_file_path, remove_data_file_in_data_dir};
use a4n_core::error::AppError;
use a4n_core::files::paths::ProjectPaths;
use a4n_core::models::ai::{generated_video, video_gen_config};
use a4n_core::models::chapter::chapter_video_prompt;
use a4n_engine::prompt::engine::PromptEngine;
use a4n_engine::queue::priority::{TaskPriority, TaskType as QueueTaskType};
use a4n_engine::video::{
    ChatCompletionsVideoProvider, GeminiVeoVideoProvider, HttpVideoProvider, NewApiVideoProvider,
    Seedance2VideoProvider, VideoGenerationRequest, VideoGenerationResult,
    VideoReferenceMediaRequest,
};
use a4n_services::generation::RecordStartedGenerationRunInput;
use a4n_services::video::CreateVideoRecord;
use a4n_services::video::{
    PromptOutputLanguage, pick_provider_for_config, validate_video_duration_for_config_secs,
};
use sea_orm::*;
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::{Map, Value, json};
use std::collections::HashMap;
use std::path::PathBuf;
use tokio_util::sync::CancellationToken;

mod boundary_frames;
mod config_state;
mod contracts;
mod first_frame_chain;
mod job_lifecycle;
pub(crate) mod media;
mod prompt_chain;
mod provider_request;
mod storyboard_frames;

pub use self::contracts::{
    GenerateCanvasVideoInput, GenerateChapterVideoInput, GenerateChapterVideosBatchInput,
    GenerateChapterVideosBatchResult, GenerateReferenceMediaVideoInput, GeneratedVideoSummary,
    PreviewChapterVideoPromptOverrideInput, ReferenceMediaVideoInput,
    VideoGenerationProgressCallback, VideoGenerationTaskScope,
};
pub(crate) use self::contracts::{VideoGenerationProgress, video_to_summary};

const VIDEO_GENERATION_PROGRESS_TOTAL: u32 = 7;
const VIDEO_JOB_POLL_INTERVAL_SECS: u64 = 8;
const VIDEO_JOB_MAX_WAIT_SECS: u64 = 30 * 60;
const VIDEO_JOB_MAX_CONSECUTIVE_POLL_ERRORS: u32 = 3;
const OPENAI_COMPATIBLE_VIDEO_BACKEND: &str = "openai-compatible-video";
const OPENAI_CHAT_VIDEO_BACKEND: &str = "openai-chat-video";
const NEWAPI_VIDEO_BACKEND: &str = "newapi-video";
const GEMINI_VEO_VIDEO_BACKEND: &str = "gemini-veo-video";
const SEEDANCE2_VIDEO_BACKEND: &str = "seedance2-video";
const CLIPROXY_OPENAI_VIDEO_BACKEND: &str = "cliproxy-openai-video";
const MAX_CLIPROXY_VIDEO_PROMPT_CHARS: usize = 4096;
const MAX_CLIPROXY_VIDEO_REFERENCE_IMAGES: usize = 7;
const MAX_SEEDANCE2_IMAGE_FILES: usize = 9;
const MAX_SEEDANCE2_VIDEO_FILES: usize = 3;
const MAX_SEEDANCE2_AUDIO_FILES: usize = 3;
const VIDEO_PROVIDER_VISUAL_PREVIEW_PLACEHOLDER: &str = "[主体定义与素材绑定]\n【请求预览占位】当前视频 visual_prompt 未完整填写。真实生成前需要补全主体、输入帧或参考图用途、人物站位、场景锚点、构图和光影约束；此占位仅用于展示将发送给视频模型的请求结构。\n\n[片段内容以及前因后果]\n【请求预览占位】真实生成前需要补全片段承接状态、剧情动作、情绪转折和结尾交接状态。\n\n[镜头分镜]\n镜头 1：【请求预览占位】补全起始镜头，交代主体、景别、空间锚点和起始动作。\n镜头 2：【请求预览占位】补全收束镜头，交代动作桥、反应和结尾状态。\n\n[画面风格描述]\n【请求预览占位】真实生成前需要补全画面风格、光线、色彩、镜头质感和材质细节。\n\n[连续性与结尾状态]\n【请求预览占位】真实生成前需要补全人物身份、服装、道具、空间关系、光源方向和可承接尾态。\n\n[限制与避免项]\n【请求预览占位】不要配背景音乐，不要字幕、水印、角色变形、空间跳变或光影不连续。";
const VIDEO_PROVIDER_MOTION_PREVIEW_PLACEHOLDER: &str = "[运镜与动作]\n【请求预览占位】当前视频 motion_prompt 未完整填写。真实生成前需要补全机位、镜头运动、速度节奏、人物动作和转场方式；此占位仅用于展示将发送给视频模型的请求结构。\n\n[对白与表演]\n【请求预览占位】真实生成前需要补全对白意图、语气、口型、手势和听者反应；无对白时明确写无对白。";
const REFERENCE_MEDIA_PREVIEW_PROMPT_PLACEHOLDER: &str = "【请求预览占位】当前参考媒体 prompt 未完整填写。真实生成前需要补全整体画面、动作、风格、镜头调度和每个参考媒体的用途；此占位仅用于展示将发送给视频模型的请求结构。";
#[derive(Debug, Clone, Copy)]
struct ProviderPromptImageBinding {
    has_first_frame_image: bool,
    has_last_frame_image: bool,
    reference_image_count: usize,
    has_storyboard_reference_image: bool,
}
const PROVIDER_REQUEST_SNAPSHOT_METADATA_KEY: &str = "provider_request_snapshot";
const CHAPTER_VIDEO_PROVIDER_PROMPT_NAME: &str = "chapter_video_provider_prompt";
const CHAPTER_VIDEO_FIRST_FRAME_PROVIDER_PROMPT_NAME: &str =
    "chapter_video_first_frame_provider_prompt";

pub fn has_active_provider_job(provider_job_id: Option<&str>, status: &str) -> bool {
    lifecycle_has_active_provider_job(provider_job_id, status)
}

#[derive(Clone)]
struct ChapterVideoProviderRequestContext {
    prompt: chapter_video_prompt::Model,
    cfg: video_gen_config::Model,
    request: VideoGenerationRequest,
    prompt_reference_image_ids: Option<serde_json::Value>,
    provider_reference_image_ids: Option<serde_json::Value>,
    prompt_source: Option<serde_json::Value>,
    workflow_mode: String,
    source_prompt_type: Option<String>,
    boundary_frame_ids: BoundaryFrameImageIds,
    prompt_reference_image_count: usize,
    provider_reference_image_count: usize,
    first_frame_image_url_available: bool,
    last_frame_image_url_available: bool,
    provider_input_note: Option<String>,
}

#[derive(Debug, Clone, Copy)]
enum VideoRequestImageMode {
    ProviderImageInputs,
    PreviewPlaceholders,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum VideoProviderRequestPurpose {
    Generate,
    Preview,
}

pub async fn generate_chapter_video(
    state: &AppState,
    input: GenerateChapterVideoInput,
    progress: Option<VideoGenerationProgressCallback<'_>>,
    cancel_token: CancellationToken,
) -> Result<GeneratedVideoSummary, String> {
    let mut progress = VideoGenerationProgress::new(progress, cancel_token);
    generate_chapter_video_inner(state, &mut progress, input).await
}

pub async fn generate_chapter_videos_batch(
    state: &AppState,
    input: GenerateChapterVideosBatchInput,
    mut progress: Option<VideoGenerationProgressCallback<'_>>,
    cancel_token: CancellationToken,
) -> Result<GenerateChapterVideosBatchResult, String> {
    let prompt_ids = validate_chapter_video_batch_prompts(state, &input).await?;
    let item_total = VIDEO_GENERATION_PROGRESS_TOTAL;
    let total = (prompt_ids.len() as u32).saturating_mul(item_total).max(1);
    let mut videos = Vec::new();
    let mut failed_count = 0usize;
    let mut pending_count = 0usize;
    for (index, prompt_id) in prompt_ids.iter().copied().enumerate() {
        if cancel_token.is_cancelled() {
            return Err("视频批量生成已停止".to_string());
        }
        let base = index as u32 * item_total;
        if let Some(reporter) = progress.as_deref_mut() {
            reporter(
                base,
                total,
                &format!("正在生成第 {}/{} 条视频", index + 1, prompt_ids.len()),
            )
            .map_err(|err| err.to_string())?;
        }
        let video = {
            let mut mapped_progress = |current: u32, child_total: u32, message: &str| {
                let child_total = child_total.max(1);
                let scaled_current = if child_total == item_total {
                    current.min(child_total)
                } else {
                    ((u64::from(current.min(child_total)) * u64::from(item_total))
                        / u64::from(child_total)) as u32
                };
                if let Some(reporter) = progress.as_deref_mut() {
                    reporter(
                        (base + scaled_current).min(total),
                        total,
                        &format!("第 {}/{} 条视频：{}", index + 1, prompt_ids.len(), message),
                    )?;
                }
                Ok(())
            };
            generate_chapter_video(
                state,
                GenerateChapterVideoInput {
                    user_id: input.user_id,
                    project_id: input.project_id,
                    chapter_id: input.chapter_id,
                    prompt_id,
                    config_id: input.config_id,
                },
                Some(&mut mapped_progress),
                cancel_token.clone(),
            )
            .await?
        };
        if video.status == "failed" {
            failed_count += 1;
        }
        if has_active_provider_job(video.provider_job_id.as_deref(), &video.status) {
            pending_count += 1;
        }
        videos.push(video);
        if let Some(reporter) = progress.as_deref_mut() {
            reporter(
                ((index as u32 + 1) * item_total).min(total),
                total,
                &format!("第 {}/{} 条视频处理完成", index + 1, prompt_ids.len()),
            )
            .map_err(|err| err.to_string())?;
        }
    }
    Ok(GenerateChapterVideosBatchResult {
        success_count: videos.len().saturating_sub(failed_count),
        failed_count,
        pending_count,
        videos,
    })
}

pub async fn chapter_video_generation_task_scope(
    state: &AppState,
    input: &GenerateChapterVideoInput,
) -> Result<VideoGenerationTaskScope, String> {
    video_generation_task_scope(state, input.user_id, input.project_id).await
}

pub async fn chapter_videos_batch_generation_task_scope(
    state: &AppState,
    input: &GenerateChapterVideosBatchInput,
) -> Result<VideoGenerationTaskScope, String> {
    access::verify_project_editor_string(state, input.user_id, input.project_id).await?;
    let prompt_count = input.prompt_ids.len().max(1);
    Ok(VideoGenerationTaskScope {
        project_id: input.project_id,
        total: (prompt_count as u32).saturating_mul(VIDEO_GENERATION_PROGRESS_TOTAL),
    })
}

async fn validate_chapter_video_batch_prompts(
    state: &AppState,
    input: &GenerateChapterVideosBatchInput,
) -> Result<Vec<i64>, String> {
    if input.prompt_ids.is_empty() {
        return Err("批量生成视频至少需要选择一条视频提示词".to_string());
    }
    video_generation_task_scope(state, input.user_id, input.project_id).await?;
    let mut unique_prompt_ids = input.prompt_ids.clone();
    unique_prompt_ids.sort_unstable();
    unique_prompt_ids.dedup();
    if unique_prompt_ids.len() != input.prompt_ids.len() {
        return Err("批量生成视频 prompt_ids 不能重复".to_string());
    }
    let rows = chapter_video_prompt::Entity::find()
        .filter(chapter_video_prompt::Column::Id.is_in(unique_prompt_ids.iter().copied()))
        .order_by_asc(chapter_video_prompt::Column::ShotNumber)
        .order_by_asc(chapter_video_prompt::Column::Id)
        .all(state.db.as_ref())
        .await
        .map_err(|err| err.to_string())?;
    if rows.len() != unique_prompt_ids.len() {
        return Err("部分视频提示词不存在，不能批量生成视频".to_string());
    }
    for row in &rows {
        if row.project_id != input.project_id || row.chapter_id != input.chapter_id {
            return Err(format!("视频提示词 #{} 不属于当前项目章节", row.id));
        }
        if row.prompt_type != "video_prompt" {
            return Err(format!("视频提示词 #{} 不是 video_prompt 类型", row.id));
        }
        ensure_provider_generation_workflow(row)?;
        if row.status == "running" {
            return Err(format!("视频提示词 #{} 正在运行，不能批量生成视频", row.id));
        }
        if row.generated_video_id.is_some() {
            return Err(format!(
                "视频提示词 #{} 已关联视频，不能批量生成视频",
                row.id
            ));
        }
    }
    Ok(rows.into_iter().map(|row| row.id).collect())
}

async fn video_generation_task_scope(
    state: &AppState,
    user_id: i64,
    project_id: i64,
) -> Result<VideoGenerationTaskScope, String> {
    access::verify_project_editor_string(state, user_id, project_id).await?;
    Ok(VideoGenerationTaskScope {
        project_id,
        total: VIDEO_GENERATION_PROGRESS_TOTAL,
    })
}

async fn generate_chapter_video_inner(
    state: &AppState,
    progress: &mut VideoGenerationProgress<'_>,
    input: GenerateChapterVideoInput,
) -> Result<GeneratedVideoSummary, String> {
    let GenerateChapterVideoInput {
        user_id,
        project_id,
        chapter_id,
        prompt_id,
        config_id,
    } = input;
    progress.progress(
        0,
        VIDEO_GENERATION_PROGRESS_TOTAL,
        "正在检查视频提示词和配置",
    )?;
    progress.check_cancelled("视频生成已停止")?;
    video_generation_task_scope(state, user_id, project_id).await?;
    let request_context = build_chapter_video_provider_request(
        state,
        project_id,
        chapter_id,
        prompt_id,
        user_id,
        config_id,
        VideoProviderRequestPurpose::Generate,
        VideoRequestImageMode::ProviderImageInputs,
        None,
    )
    .await?;
    let provider = pick_provider_for_config(&request_context.cfg)?;
    ensure_video_capacity(
        state,
        request_context.cfg.id,
        request_context.cfg.max_concurrent,
    )
    .await?;

    progress.progress(2, VIDEO_GENERATION_PROGRESS_TOTAL, "正在创建视频生成记录")?;
    progress.check_cancelled("视频生成已停止")?;
    let provider_request_snapshot = chapter_video_provider_request_snapshot(&request_context)?;
    let metadata = json!({
        "config_id": request_context.cfg.id,
        "prompt_id": request_context.prompt.id,
        "shot_number": request_context.prompt.shot_number,
        "source": request_context.prompt_source,
        "source_prompt_type": request_context.source_prompt_type,
        "prompt_reference_image_ids": request_context.prompt_reference_image_ids,
        "prompt_reference_image_count": request_context.prompt_reference_image_count,
        "provider_reference_image_ids": request_context.provider_reference_image_ids,
        "provider_reference_image_count": request_context.provider_reference_image_count,
        "start_frame_image_id": request_context.boundary_frame_ids.first,
        "boundary_frame_image_ids": {
            "first": request_context.boundary_frame_ids.first,
            "last": request_context.boundary_frame_ids.last
        },
        "boundary_frame_image_count": usize::from(request_context.boundary_frame_ids.first.is_some())
            + usize::from(request_context.boundary_frame_ids.last.is_some()),
        "boundary_frame_image_url_count": usize::from(request_context.first_frame_image_url_available)
            + usize::from(request_context.last_frame_image_url_available),
        "boundary_frame_request": {
            "first_frame_image_url": request_context.first_frame_image_url_available,
            "last_frame_image_url": request_context.last_frame_image_url_available,
        },
        "provider_request_snapshot": provider_request_snapshot,
        "runtime_config_snapshot": crate::runtime_config::redacted_current_snapshot_value(),
    });
    let video_record = CreateVideoRecord {
        project_id,
        chapter_id: Some(chapter_id),
        video_type: "chapter_video".into(),
        workflow_mode: Some(request_context.workflow_mode.clone()),
        prompt: request_context.request.prompt.clone(),
        negative_prompt: request_context.request.negative_prompt.clone(),
        model: request_context.cfg.model.clone(),
        width: request_context.request.width,
        height: request_context.request.height,
        duration_secs: request_context.request.duration_secs as f32,
        fps: request_context.request.fps,
        source_video_id: None,
        source_image_id: request_context.boundary_frame_ids.first,
        provider_job_id: None,
        metadata_json: Some(metadata),
    };
    let generation_run = started_video_generation_run_input(
        &video_record,
        "generate_chapter_video",
        "章节视频 provider 生成",
        request_context.cfg.backend.clone(),
        Some("chapter_video_prompt"),
        provider_request_snapshot,
    );
    let video = state
        .video
        .create_prompt_video_record_with_generation_run(
            video_record,
            generation_run,
            request_context.prompt.id,
        )
        .await
        .map_err(|e| e.to_string())?;
    if let Err(msg) = ensure_created_video_capacity(
        state,
        request_context.cfg.id,
        request_context.cfg.max_concurrent,
        video.id,
    )
    .await
    {
        return mark_video_generation_failed(
            state,
            video.id,
            Some(request_context.prompt.id),
            &msg,
        )
        .await;
    }

    let req = request_context.request.clone();
    let prompt_length_context = video_request_prompt_length_context(&req);
    progress.progress(3, VIDEO_GENERATION_PROGRESS_TOTAL, "正在调用视频模型")?;
    if let Err(msg) = progress.check_cancelled("视频生成已停止") {
        return mark_video_generation_failed(
            state,
            video.id,
            Some(request_context.prompt.id),
            &msg,
        )
        .await;
    }
    let result = run_queued_request_with_cancel(
        state,
        QueueTaskType::VideoGeneration,
        TaskPriority::Medium,
        Some(project_id),
        progress.cancel_token(),
        || async move { provider.generate(req).await.map_err(|e| e.to_string()) },
    )
    .await
    .map_err(|err| append_video_request_prompt_length_context(err, &prompt_length_context));
    handle_video_provider_result(
        state,
        progress,
        video,
        &request_context.cfg,
        result,
        VideoProviderResultHandling {
            prompt_id: Some(request_context.prompt.id),
            completed_cancel_message: Some("视频生成已停止"),
            async_job_progress_message: "视频模型已提交异步任务，开始等待生成结果",
            missing_result_message: "视频 provider 未返回文件或任务 ID",
        },
    )
    .await
}

fn video_request_prompt_length_context(request: &VideoGenerationRequest) -> String {
    format!(
        "实际发送 prompt：{} 字符 / {} UTF-8 字节",
        request.prompt.chars().count(),
        request.prompt.len()
    )
}

fn append_video_request_prompt_length_context(err: String, context: &str) -> String {
    if err.contains(context) {
        err
    } else {
        format!("{err}；{context}")
    }
}

pub async fn preview_chapter_video_request(
    state: &AppState,
    input: GenerateChapterVideoInput,
    prompt_override: Option<PreviewChapterVideoPromptOverrideInput>,
) -> Result<ModelRequestPreview, String> {
    chapter_video_generation_task_scope(state, &input).await?;
    let context = build_chapter_video_provider_request(
        state,
        input.project_id,
        input.chapter_id,
        input.prompt_id,
        input.user_id,
        input.config_id,
        VideoProviderRequestPurpose::Preview,
        VideoRequestImageMode::PreviewPlaceholders,
        prompt_override,
    )
    .await?;
    let (request_kind, request_title, request_note, request) =
        video_provider_preview_payload(&context)?;
    Ok(model_request_preview(
        "chapter_video_generation",
        "章节视频模型请求",
        vec![provider_request_step(
            "chapter_video",
            request_title,
            request_kind,
            Some("chapter_video_prompt".to_string()),
            "ready",
            Some(request_note),
            &request,
        )?],
    ))
}

pub async fn preview_video_provider_request_by_video_id(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    chapter_id: i64,
    video_id: i64,
) -> Result<ModelRequestPreview, String> {
    let video = generated_video::Entity::find_by_id(video_id)
        .one(state.db.as_ref())
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "视频记录不存在".to_string())?;
    access::verify_project_editor_string(state, user_id, video.project_id).await?;
    resolve_project_chapter_video_source(
        state,
        project_id,
        chapter_id,
        "预览历史视频 provider 请求",
        false,
    )
    .await?;
    if video.project_id != project_id || video.chapter_id != Some(chapter_id) {
        return Err("视频记录不属于该章节".to_string());
    }
    validate_video_provider_request_snapshot_owner(&video)?;
    model_request_preview_from_provider_snapshot(
        &video,
        "video_provider_request_record",
        "历史视频模型请求",
    )
}

pub async fn preview_reference_media_video_request(
    state: &AppState,
    input: GenerateReferenceMediaVideoInput,
) -> Result<ModelRequestPreview, String> {
    let GenerateReferenceMediaVideoInput {
        user_id,
        project_id,
        chapter_id,
        input,
    } = input;
    video_generation_task_scope(state, user_id, project_id).await?;
    let context = build_reference_media_provider_request(
        state,
        project_id,
        chapter_id,
        input,
        VideoProviderRequestPurpose::Preview,
    )
    .await?;
    reference_media_provider_request_preview(
        "reference_media_video_generation",
        "参考媒体视频模型请求",
        "当前表单",
        &ReferenceMediaProviderRequestPreviewContext {
            request: context.request,
            config_id: Some(context.cfg.id),
            provider_backend: Some(context.cfg.backend),
            provider_input_note: context.provider_input_note,
        },
    )
}

pub async fn preview_canvas_video_request(
    state: &AppState,
    input: GenerateCanvasVideoInput,
) -> Result<ModelRequestPreview, String> {
    let context = build_canvas_video_provider_request(
        state,
        input,
        VideoProviderRequestPurpose::Preview,
        VideoRequestImageMode::PreviewPlaceholders,
    )
    .await?;
    canvas_video_provider_request_preview(&context)
}

pub async fn generate_canvas_video(
    state: &AppState,
    input: GenerateCanvasVideoInput,
    progress: Option<VideoGenerationProgressCallback<'_>>,
    cancel_token: CancellationToken,
) -> Result<GeneratedVideoSummary, String> {
    let mut progress = VideoGenerationProgress::new(progress, cancel_token);
    progress.progress(1, VIDEO_GENERATION_PROGRESS_TOTAL, "正在校验画布视频节点")?;
    progress.check_cancelled("画布视频生成已停止")?;
    let context = build_canvas_video_provider_request(
        state,
        input,
        VideoProviderRequestPurpose::Generate,
        VideoRequestImageMode::ProviderImageInputs,
    )
    .await?;
    let provider = pick_provider_for_config(&context.cfg)?;
    ensure_video_capacity(state, context.cfg.id, context.cfg.max_concurrent).await?;

    progress.progress(2, VIDEO_GENERATION_PROGRESS_TOTAL, "正在创建画布视频记录")?;
    progress.check_cancelled("画布视频生成已停止")?;
    let provider_request_snapshot = canvas_video_provider_request_snapshot(&context)?;
    let metadata = json!({
        "canvas": {
            "graph_id": context.graph_id,
            "node_id": context.node_id,
            "workflow_run_id": context.workflow_run_id,
            "source_node_ids": context.source_node_ids,
        },
        "config_id": context.cfg.id,
        "first_frame_image_id": context.first_frame_image_id,
        "last_frame_image_id": context.last_frame_image_id,
        "reference_image_ids": context.reference_image_ids,
        "reference_image_count": context.reference_image_ids.len(),
        "provider_request_snapshot": provider_request_snapshot,
        "runtime_config_snapshot": crate::runtime_config::redacted_current_snapshot_value(),
    });
    let video_record = CreateVideoRecord {
        project_id: context.project_id,
        chapter_id: None,
        video_type: "canvas_video".to_string(),
        workflow_mode: Some("canvas_video".to_string()),
        prompt: context.request.prompt.clone(),
        negative_prompt: context.request.negative_prompt.clone(),
        model: context.cfg.model.clone(),
        width: context.request.width,
        height: context.request.height,
        duration_secs: context.request.duration_secs as f32,
        fps: context.request.fps,
        source_video_id: None,
        source_image_id: context.first_frame_image_id,
        provider_job_id: None,
        metadata_json: Some(metadata),
    };
    let generation_run = started_video_generation_run_input(
        &video_record,
        "generate_canvas_video",
        "画布视频 provider 生成",
        context.cfg.backend.clone(),
        Some("canvas_video_prompt"),
        provider_request_snapshot,
    );
    let video = state
        .video
        .create_video_record_with_generation_run(video_record, generation_run)
        .await
        .map_err(|e| e.to_string())?;
    if let Err(msg) =
        ensure_created_video_capacity(state, context.cfg.id, context.cfg.max_concurrent, video.id)
            .await
    {
        return mark_video_generation_failed(state, video.id, None, &msg).await;
    }

    let req = context.request.clone();
    let prompt_length_context = video_request_prompt_length_context(&req);
    progress.progress(3, VIDEO_GENERATION_PROGRESS_TOTAL, "正在调用视频模型")?;
    if let Err(msg) = progress.check_cancelled("画布视频生成已停止") {
        return mark_video_generation_failed(state, video.id, None, &msg).await;
    }
    let result = run_queued_request_with_cancel_and_workflow(
        state,
        QueueTaskType::VideoGeneration,
        TaskPriority::Medium,
        Some(context.project_id),
        context.workflow_run_id,
        progress.cancel_token(),
        || async move { provider.generate(req).await.map_err(|e| e.to_string()) },
    )
    .await
    .map_err(|err| append_video_request_prompt_length_context(err, &prompt_length_context));

    handle_canvas_video_provider_result(state, &mut progress, video, &context.cfg, result).await
}

pub async fn generate_reference_media_video(
    state: &AppState,
    input: GenerateReferenceMediaVideoInput,
    progress: Option<VideoGenerationProgressCallback<'_>>,
    cancel_token: CancellationToken,
) -> Result<GeneratedVideoSummary, String> {
    let mut progress = VideoGenerationProgress::new(progress, cancel_token);
    generate_reference_media_video_inner(state, &mut progress, input).await
}

pub async fn reference_media_video_generation_task_scope(
    state: &AppState,
    input: &GenerateReferenceMediaVideoInput,
) -> Result<VideoGenerationTaskScope, String> {
    video_generation_task_scope(state, input.user_id, input.project_id).await
}

async fn handle_video_provider_result(
    state: &AppState,
    progress: &mut VideoGenerationProgress<'_>,
    video: generated_video::Model,
    cfg: &video_gen_config::Model,
    result: Result<VideoGenerationResult, String>,
    handling: VideoProviderResultHandling<'_>,
) -> Result<GeneratedVideoSummary, String> {
    match result {
        Ok(result) if !result.bytes.is_empty() => {
            if let Some(message) = handling.completed_cancel_message
                && let Err(msg) = progress.check_cancelled(message)
            {
                return mark_video_generation_failed(state, video.id, handling.prompt_id, &msg)
                    .await;
            }
            progress.progress(
                6,
                VIDEO_GENERATION_PROGRESS_TOTAL,
                "视频模型已返回，正在保存文件",
            )?;
            let updated =
                save_completed_video_result(state, &video, handling.prompt_id, result).await?;
            progress.progress(7, VIDEO_GENERATION_PROGRESS_TOTAL, "视频文件已保存")?;
            Ok(video_to_summary(&updated))
        }
        Ok(result) => {
            if let Some(job_id) =
                normalized_provider_job_id(result.provider_job_id.as_deref()).map(str::to_string)
            {
                progress.progress(
                    4,
                    VIDEO_GENERATION_PROGRESS_TOTAL,
                    handling.async_job_progress_message,
                )?;
                let updated = state
                    .video
                    .mark_job_and_update_prompt(
                        video.id,
                        &job_id,
                        Some(result.metadata),
                        handling.prompt_id,
                    )
                    .await
                    .map_err(|e| e.to_string())?;
                let final_video =
                    wait_for_video_job_completion(state, progress, updated, cfg, &job_id).await?;
                Ok(video_to_summary(&final_video))
            } else {
                mark_video_generation_failed(
                    state,
                    video.id,
                    handling.prompt_id,
                    handling.missing_result_message,
                )
                .await
            }
        }
        Err(err) => mark_video_generation_failed(state, video.id, handling.prompt_id, &err).await,
    }
}

async fn handle_canvas_video_provider_result(
    state: &AppState,
    progress: &mut VideoGenerationProgress<'_>,
    video: generated_video::Model,
    cfg: &video_gen_config::Model,
    result: Result<VideoGenerationResult, String>,
) -> Result<GeneratedVideoSummary, String> {
    match result {
        Ok(result) if !result.bytes.is_empty() => {
            if let Err(msg) = progress.check_cancelled("画布视频生成已停止") {
                return mark_video_generation_failed(state, video.id, None, &msg).await;
            }
            progress.progress(
                6,
                VIDEO_GENERATION_PROGRESS_TOTAL,
                "视频模型已返回，正在保存画布视频文件",
            )?;
            let updated = save_completed_canvas_video_result(state, &video, result).await?;
            progress.progress(7, VIDEO_GENERATION_PROGRESS_TOTAL, "画布视频文件已保存")?;
            Ok(video_to_summary(&updated))
        }
        Ok(result) => {
            if let Some(job_id) =
                normalized_provider_job_id(result.provider_job_id.as_deref()).map(str::to_string)
            {
                progress.progress(
                    4,
                    VIDEO_GENERATION_PROGRESS_TOTAL,
                    "视频模型已提交异步任务，画布视频记录已标记为运行中",
                )?;
                let updated = state
                    .video
                    .mark_job(video.id, &job_id, Some(result.metadata))
                    .await
                    .map_err(|e| e.to_string())?;
                let final_video = wait_for_canvas_graph_video_job_completion(
                    state, progress, updated, cfg, &job_id,
                )
                .await?;
                Ok(video_to_summary(&final_video))
            } else {
                mark_video_generation_failed(
                    state,
                    video.id,
                    None,
                    "画布视频 provider 未返回文件或任务 ID",
                )
                .await
            }
        }
        Err(err) => mark_video_generation_failed(state, video.id, None, &err).await,
    }
}

async fn save_completed_canvas_video_result(
    state: &AppState,
    video: &generated_video::Model,
    result: VideoGenerationResult,
) -> Result<generated_video::Model, String> {
    if result.bytes.is_empty() {
        return state
            .video
            .mark_failed_and_update_prompt(
                video.id,
                None,
                "画布视频 provider 返回了空文件，无法保存生成结果",
            )
            .await
            .map_err(|e| e.to_string());
    }
    let path = match save_canvas_video_bytes(
        state,
        video.project_id,
        video.id,
        &result.bytes,
        &result.format,
    ) {
        Ok(path) => path,
        Err(msg) => {
            return state
                .video
                .mark_failed_and_update_prompt(video.id, None, &msg)
                .await
                .map_err(|e| e.to_string());
        }
    };
    let video_path_str =
        data_dir_relative_file_path(&state.config.data_dir, &path, "画布视频文件")?;
    let updated = match state
        .video
        .mark_completed_and_link_prompt(
            video.id,
            &video_path_str,
            None,
            result.bytes.len() as i64,
            Some(json!({
                "provider": result.metadata,
                "canvas_video": true,
            })),
            None,
        )
        .await
    {
        Ok(updated) => updated,
        Err(err) => {
            let _ = remove_data_file_in_data_dir(
                &state.config.data_dir,
                &video_path_str,
                "画布视频文件",
            );
            return Err(format!("画布视频已生成但记录完成状态失败：{err}"));
        }
    };
    state
        .generation_run
        .complete_runs_for_target(
            "generated_video",
            video.id,
            Some(json!({
                "generated_video_id": video.id,
                "file_path": updated.file_path,
                "thumbnail_path": updated.thumbnail_path,
                "file_size": updated.file_size,
                "status": updated.status,
                "provider_result": updated.metadata_json,
            })),
        )
        .await
        .map_err(|err| err.to_string())
        .and_then(|runs| {
            ensure_provider_generation_run_updated(video.id, runs.len(), "完成 provider 视频生成")
        })?;
    Ok(updated)
}

fn save_canvas_video_bytes(
    state: &AppState,
    project_id: i64,
    video_id: i64,
    bytes: &[u8],
    format: &str,
) -> Result<PathBuf, String> {
    let extension = required_video_file_extension(format)?;
    let dir = ProjectPaths::new(&state.config.data_dir, project_id).videos_dir();
    std::fs::create_dir_all(&dir)
        .map_err(|err| format!("无法创建画布视频目录 {}：{err}", dir.display()))?;
    let path = dir.join(format!("canvas_video_{video_id}.{extension}"));
    std::fs::write(&path, bytes).map_err(|err| format!("无法写入画布视频：{err}"))?;
    Ok(path)
}

async fn mark_video_generation_failed(
    state: &AppState,
    video_id: i64,
    prompt_id: Option<i64>,
    message: &str,
) -> Result<GeneratedVideoSummary, String> {
    let updated = state
        .video
        .mark_failed_and_update_prompt(video_id, prompt_id, message)
        .await
        .map_err(|e| e.to_string())?;
    fail_video_generation_runs(state, video_id, message).await?;
    Ok(video_to_summary(&updated))
}

fn started_video_generation_run_input(
    video: &CreateVideoRecord,
    step_key: &str,
    title: &str,
    provider: String,
    prompt_template_name: Option<&str>,
    request_json: Value,
) -> RecordStartedGenerationRunInput {
    let prompt_template_names_json = prompt_template_name.map(|name| json!([name]));
    RecordStartedGenerationRunInput {
        project_id: video.project_id,
        workflow: "video_generation".to_string(),
        step_key: step_key.to_string(),
        title: title.to_string(),
        model_kind: "video".to_string(),
        provider: Some(provider),
        model: Some(video.model.clone()),
        prompt_template_names_json,
        prompt_template_hashes_json: None,
        rendered_messages_json: None,
        rendered_prompts_json: Some(json!({
            "prompt": video.prompt,
            "negative_prompt": video.negative_prompt,
        })),
        input_variables_json: None,
        source_refs_json: video_generation_source_refs(video.metadata_json.as_ref()),
        media_refs_json: video_generation_media_refs(video.metadata_json.as_ref()),
        request_json: Some(request_json),
        targets: Vec::new(),
    }
}

pub(super) async fn fail_video_generation_runs(
    state: &AppState,
    video_id: i64,
    message: &str,
) -> Result<(), String> {
    state
        .generation_run
        .fail_runs_for_target("generated_video", video_id, message.to_string(), None)
        .await
        .map_err(|err| err.to_string())
        .and_then(|runs| {
            ensure_provider_generation_run_updated(
                video_id,
                runs.len(),
                "标记 provider 视频生成失败",
            )
        })?;
    Ok(())
}

pub(super) fn ensure_provider_generation_run_updated(
    video_id: i64,
    updated_run_count: usize,
    action: &str,
) -> Result<(), String> {
    if updated_run_count == 0 {
        return Err(format!(
            "{action}失败：视频 #{video_id} 缺少可更新的 generation_run"
        ));
    }
    Ok(())
}

fn video_generation_source_refs(metadata_json: Option<&Value>) -> Option<Value> {
    metadata_json
        .and_then(|metadata| metadata.get("source"))
        .cloned()
        .map(|source| json!({ "source": source }))
}

fn video_generation_media_refs(metadata_json: Option<&Value>) -> Option<Value> {
    let metadata = metadata_json?;
    let mut media = Map::new();
    copy_json_field(metadata, &mut media, "material_reference_image_ids");
    copy_json_field(metadata, &mut media, "material_reference_image_count");
    copy_json_field(metadata, &mut media, "prompt_reference_image_ids");
    copy_json_field(metadata, &mut media, "prompt_reference_image_count");
    copy_json_field(metadata, &mut media, "provider_reference_image_ids");
    copy_json_field(metadata, &mut media, "provider_reference_image_count");
    copy_json_field(metadata, &mut media, "start_frame_image_id");
    copy_json_field(metadata, &mut media, "boundary_frame_image_ids");
    copy_json_field(metadata, &mut media, "boundary_frame_image_count");
    copy_json_field(metadata, &mut media, "reference_media");
    if media.is_empty() {
        None
    } else {
        Some(Value::Object(media))
    }
}

fn copy_json_field(source: &Value, target: &mut Map<String, Value>, key: &str) {
    if let Some(value) = source.get(key) {
        target.insert(key.to_string(), value.clone());
    }
}

struct ReferenceMediaProviderRequestContext {
    cfg: video_gen_config::Model,
    request: VideoGenerationRequest,
    provider_input_note: Option<String>,
}

struct CanvasVideoProviderRequestContext {
    project_id: i64,
    graph_id: i64,
    node_id: i64,
    workflow_run_id: Option<i64>,
    source_node_ids: Vec<i64>,
    cfg: video_gen_config::Model,
    request: VideoGenerationRequest,
    provider_input_note: Option<String>,
    first_frame_image_id: Option<i64>,
    last_frame_image_id: Option<i64>,
    reference_image_ids: Vec<i64>,
}

struct VideoProviderResultHandling<'a> {
    prompt_id: Option<i64>,
    completed_cancel_message: Option<&'a str>,
    async_job_progress_message: &'a str,
    missing_result_message: &'a str,
}

#[derive(Debug)]
struct ReferenceMediaProviderRequestPreviewContext {
    request: VideoGenerationRequest,
    config_id: Option<i64>,
    provider_backend: Option<String>,
    provider_input_note: Option<String>,
}

async fn build_canvas_video_provider_request(
    state: &AppState,
    input: GenerateCanvasVideoInput,
    purpose: VideoProviderRequestPurpose,
    image_mode: VideoRequestImageMode,
) -> Result<CanvasVideoProviderRequestContext, String> {
    access::verify_project_editor_string(state, input.user_id, input.project_id).await?;
    let project = state
        .project
        .get(input.project_id)
        .await
        .map_err(|e| e.to_string())?;
    if project.project_kind != "video" || project.source_mode != "freeform" {
        return Err("GenerateVideoNode 当前只支持自由视频项目".to_string());
    }
    let cfg = resolve_video_config(state, input.config_id).await?;
    let duration_secs = input.duration_secs.unwrap_or(5);
    match purpose {
        VideoProviderRequestPurpose::Generate => {
            validate_video_duration_for_config_secs(&cfg, duration_secs)?;
        }
        VideoProviderRequestPurpose::Preview => {}
    }
    let width = input.width.unwrap_or(cfg.default_width);
    let height = input.height.unwrap_or(cfg.default_height);
    if width <= 0 || height <= 0 {
        return Err("画布视频宽高必须为正整数".to_string());
    }
    let fps = input.fps.unwrap_or(cfg.default_fps);
    if fps <= 0 {
        return Err("画布视频 fps 必须为正整数".to_string());
    }
    let prompt = input.prompt.trim().to_string();
    if prompt.is_empty() {
        return Err("GenerateVideoNode 缺少有效 prompt 输入".to_string());
    }
    let mut provider_input_note = None;
    let image_transport = default_canvas_video_image_transport();
    let image_binding =
        canvas_video_provider_image_binding(&cfg, &input, &mut provider_input_note)?;
    let first_frame_image_url = match image_mode {
        VideoRequestImageMode::ProviderImageInputs => {
            reference_image_provider_input_by_id(
                state,
                input.user_id,
                input.project_id,
                0,
                image_binding.first_frame_image_id,
                image_transport,
            )
            .await?
        }
        VideoRequestImageMode::PreviewPlaceholders => {
            let (url, notes) = reference_image_preview_placeholder_by_id(
                state,
                input.user_id,
                input.project_id,
                0,
                image_binding.first_frame_image_id,
            )
            .await?;
            for note in notes {
                append_provider_input_note(&mut provider_input_note, note);
            }
            url
        }
    };
    let last_frame_image_url = match image_mode {
        VideoRequestImageMode::ProviderImageInputs => {
            reference_image_provider_input_by_id(
                state,
                input.user_id,
                input.project_id,
                0,
                image_binding.last_frame_image_id,
                image_transport,
            )
            .await?
        }
        VideoRequestImageMode::PreviewPlaceholders => {
            let (url, notes) = reference_image_preview_placeholder_by_id(
                state,
                input.user_id,
                input.project_id,
                0,
                image_binding.last_frame_image_id,
            )
            .await?;
            for note in notes {
                append_provider_input_note(&mut provider_input_note, note);
            }
            url
        }
    };
    let reference_image_urls = match image_mode {
        VideoRequestImageMode::ProviderImageInputs => {
            reference_image_provider_inputs(
                state,
                input.user_id,
                input.project_id,
                0,
                Some(&json!(image_binding.reference_image_ids)),
                image_transport,
            )
            .await?
        }
        VideoRequestImageMode::PreviewPlaceholders => {
            let placeholders = reference_image_preview_placeholders(
                state,
                input.user_id,
                input.project_id,
                0,
                Some(&json!(image_binding.reference_image_ids)),
            )
            .await?;
            for note in placeholders.notes {
                append_provider_input_note(&mut provider_input_note, note);
            }
            placeholders.urls
        }
    };
    let request = VideoGenerationRequest {
        prompt,
        negative_prompt: input
            .negative_prompt
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string),
        width,
        height,
        duration_secs,
        fps,
        model: cfg.model.clone(),
        first_frame_image_url,
        last_frame_image_url,
        reference_image_urls,
        reference_media: None,
    };
    Ok(CanvasVideoProviderRequestContext {
        project_id: input.project_id,
        graph_id: input.graph_id,
        node_id: input.node_id,
        workflow_run_id: input.workflow_run_id,
        source_node_ids: input.source_node_ids,
        cfg,
        request,
        provider_input_note,
        first_frame_image_id: input.first_frame_image_id,
        last_frame_image_id: input.last_frame_image_id,
        reference_image_ids: image_binding.reference_image_ids,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CanvasVideoProviderImageBinding {
    first_frame_image_id: Option<i64>,
    last_frame_image_id: Option<i64>,
    reference_image_ids: Vec<i64>,
}

fn canvas_video_provider_image_binding(
    cfg: &video_gen_config::Model,
    input: &GenerateCanvasVideoInput,
    provider_input_note: &mut Option<String>,
) -> Result<CanvasVideoProviderImageBinding, String> {
    if cfg.backend != CLIPROXY_OPENAI_VIDEO_BACKEND {
        return Ok(CanvasVideoProviderImageBinding {
            first_frame_image_id: input.first_frame_image_id,
            last_frame_image_id: input.last_frame_image_id,
            reference_image_ids: input.reference_image_ids.clone(),
        });
    }

    match cliproxy_grok_reference_mode(cfg)? {
        CliproxyGrokReferenceMode::FirstFrame => Ok(CanvasVideoProviderImageBinding {
            first_frame_image_id: input.first_frame_image_id,
            last_frame_image_id: input.last_frame_image_id,
            reference_image_ids: input.reference_image_ids.clone(),
        }),
        CliproxyGrokReferenceMode::Reference => {
            let reference_image_ids = canvas_video_reference_ids_with_frame_images(
                input.first_frame_image_id,
                input.last_frame_image_id,
                &input.reference_image_ids,
            );
            if input.first_frame_image_id.is_some() || input.last_frame_image_id.is_some() {
                append_provider_input_note(
                    provider_input_note,
                    format!(
                        "CLIProxy/Grok 参考模式已将画布首帧/尾帧输入作为普通 reference_images 发送；实际参考图 ID 顺序：{}",
                        reference_image_ids
                            .iter()
                            .map(i64::to_string)
                            .collect::<Vec<_>>()
                            .join(", ")
                    ),
                );
            }
            Ok(CanvasVideoProviderImageBinding {
                first_frame_image_id: None,
                last_frame_image_id: None,
                reference_image_ids,
            })
        }
    }
}

fn canvas_video_reference_ids_with_frame_images(
    first_frame_image_id: Option<i64>,
    last_frame_image_id: Option<i64>,
    reference_image_ids: &[i64],
) -> Vec<i64> {
    let mut ids = Vec::new();
    for id in first_frame_image_id
        .into_iter()
        .chain(last_frame_image_id)
        .chain(reference_image_ids.iter().copied())
    {
        if id > 0 && !ids.contains(&id) {
            ids.push(id);
        }
    }
    ids
}

fn default_canvas_video_image_transport() -> ProviderImageInputTransport {
    if remote_temp_upload_configured() {
        ProviderImageInputTransport::RemoteTempUpload
    } else {
        ProviderImageInputTransport::InlineDataUrl
    }
}

async fn build_reference_media_provider_request(
    state: &AppState,
    project_id: i64,
    chapter_id: i64,
    input: ReferenceMediaVideoInput,
    purpose: VideoProviderRequestPurpose,
) -> Result<ReferenceMediaProviderRequestContext, String> {
    match purpose {
        VideoProviderRequestPurpose::Generate => {
            resolve_project_chapter_video_source(
                state,
                project_id,
                chapter_id,
                "生成参考媒体视频",
                true,
            )
            .await?;
        }
        VideoProviderRequestPurpose::Preview => {
            resolve_project_chapter_video_source(
                state,
                project_id,
                chapter_id,
                "预览参考媒体视频请求",
                false,
            )
            .await?;
        }
    }
    let cfg = resolve_video_config(state, input.config_id).await?;
    let mut provider_input_note = None;
    if cfg.backend != SEEDANCE2_VIDEO_BACKEND {
        let message = format!(
            "参考媒体模式必须使用 {SEEDANCE2_VIDEO_BACKEND} 视频配置，当前配置 backend={}",
            cfg.backend
        );
        if purpose == VideoProviderRequestPurpose::Generate {
            return Err(message);
        }
        append_provider_input_note(
            &mut provider_input_note,
            format!(
                "{message}；此处仅展示 Seedance 2.0 参考媒体请求结构，真实生成仍需切换到支持的配置"
            ),
        );
    }
    match purpose {
        VideoProviderRequestPurpose::Generate => {
            validate_video_duration_for_config_secs(&cfg, input.duration_secs)?;
        }
        VideoProviderRequestPurpose::Preview => {
            append_preview_duration_note(&mut provider_input_note, &cfg, input.duration_secs);
        }
    }
    let mut prompt = reference_media_prompt(&input.prompt, purpose, &mut provider_input_note)?;
    let mut image_files = reference_media_urls(
        input.image_files,
        "image_files",
        MAX_SEEDANCE2_IMAGE_FILES,
        purpose,
        &mut provider_input_note,
    )?;
    let video_files = reference_media_urls(
        input.video_files,
        "video_files",
        MAX_SEEDANCE2_VIDEO_FILES,
        purpose,
        &mut provider_input_note,
    )?;
    let audio_files = reference_media_urls(
        input.audio_files,
        "audio_files",
        MAX_SEEDANCE2_AUDIO_FILES,
        purpose,
        &mut provider_input_note,
    )?;
    if image_files.is_empty() && video_files.is_empty() && audio_files.is_empty() {
        if purpose == VideoProviderRequestPurpose::Generate {
            return Err(
                "参考媒体模式至少需要一个 image_files、video_files 或 audio_files URL".to_string(),
            );
        }
        append_provider_input_note(
            &mut provider_input_note,
            "参考媒体模式至少需要一个 image_files、video_files 或 audio_files URL；预览使用占位 image_files 展示请求结构，真实生成仍需填写可访问媒体 URL",
        );
        image_files.push(reference_media_preview_placeholder_url("image_files", 0));
    }
    for (token_prefix, count) in [
        ("image_file", image_files.len()),
        ("video_file", video_files.len()),
        ("audio_file", audio_files.len()),
    ] {
        ensure_reference_media_tokens(
            &mut prompt,
            token_prefix,
            count,
            purpose,
            &mut provider_input_note,
        )?;
    }
    let (width, height) = reference_media_dimensions_for_aspect_ratio(&input.aspect_ratio)?;
    let request = VideoGenerationRequest {
        prompt,
        negative_prompt: None,
        width,
        height,
        duration_secs: input.duration_secs,
        fps: cfg.default_fps,
        model: cfg.model.clone(),
        first_frame_image_url: None,
        last_frame_image_url: None,
        reference_image_urls: Vec::new(),
        reference_media: Some(VideoReferenceMediaRequest {
            image_files,
            video_files,
            audio_files,
        }),
    };
    Ok(ReferenceMediaProviderRequestContext {
        cfg,
        request,
        provider_input_note,
    })
}

async fn generate_reference_media_video_inner(
    state: &AppState,
    progress: &mut VideoGenerationProgress<'_>,
    input: GenerateReferenceMediaVideoInput,
) -> Result<GeneratedVideoSummary, String> {
    let GenerateReferenceMediaVideoInput {
        user_id,
        project_id,
        chapter_id,
        input,
    } = input;
    progress.progress(1, VIDEO_GENERATION_PROGRESS_TOTAL, "正在校验参考媒体请求")?;
    progress.check_cancelled("参考媒体视频生成已停止")?;
    video_generation_task_scope(state, user_id, project_id).await?;
    let context = build_reference_media_provider_request(
        state,
        project_id,
        chapter_id,
        input,
        VideoProviderRequestPurpose::Generate,
    )
    .await?;
    let provider = pick_provider_for_config(&context.cfg)?;
    ensure_video_capacity(state, context.cfg.id, context.cfg.max_concurrent).await?;

    progress.progress(2, VIDEO_GENERATION_PROGRESS_TOTAL, "正在创建视频生成记录")?;
    progress.check_cancelled("参考媒体视频生成已停止")?;
    let provider_request_snapshot =
        reference_media_provider_request_snapshot(&ReferenceMediaProviderRequestPreviewContext {
            request: context.request.clone(),
            config_id: Some(context.cfg.id),
            provider_backend: Some(context.cfg.backend.clone()),
            provider_input_note: context.provider_input_note.clone(),
        })?;
    let reference_media = context
        .request
        .reference_media
        .as_ref()
        .ok_or_else(|| "参考媒体请求缺少 reference_media".to_string())?;
    let metadata = json!({
        "config_id": context.cfg.id,
        "provider_backend": context.cfg.backend,
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
        "runtime_config_snapshot": crate::runtime_config::redacted_current_snapshot_value(),
    });
    let video_record = CreateVideoRecord {
        project_id,
        chapter_id: Some(chapter_id),
        video_type: "reference_media_video".into(),
        workflow_mode: Some("reference_media".to_string()),
        prompt: context.request.prompt.clone(),
        negative_prompt: None,
        model: context.cfg.model.clone(),
        width: context.request.width,
        height: context.request.height,
        duration_secs: context.request.duration_secs as f32,
        fps: context.request.fps,
        source_video_id: None,
        source_image_id: None,
        provider_job_id: None,
        metadata_json: Some(metadata),
    };
    let generation_run = started_video_generation_run_input(
        &video_record,
        "generate_reference_media_video",
        "参考媒体视频 provider 生成",
        context.cfg.backend.clone(),
        Some("reference_media_video"),
        provider_request_snapshot,
    );
    let video = state
        .video
        .create_video_record_with_generation_run(video_record, generation_run)
        .await
        .map_err(|e| e.to_string())?;
    if let Err(msg) =
        ensure_created_video_capacity(state, context.cfg.id, context.cfg.max_concurrent, video.id)
            .await
    {
        return mark_video_generation_failed(state, video.id, None, &msg).await;
    }

    progress.progress(3, VIDEO_GENERATION_PROGRESS_TOTAL, "正在调用 Seedance 2.0")?;
    if let Err(msg) = progress.check_cancelled("参考媒体视频生成已停止") {
        return mark_video_generation_failed(state, video.id, None, &msg).await;
    }
    let req = context.request.clone();
    let prompt_length_context = video_request_prompt_length_context(&req);
    let result = run_queued_request_with_cancel(
        state,
        QueueTaskType::VideoGeneration,
        TaskPriority::Medium,
        Some(project_id),
        progress.cancel_token(),
        || async move { provider.generate(req).await.map_err(|e| e.to_string()) },
    )
    .await
    .map_err(|err| append_video_request_prompt_length_context(err, &prompt_length_context));
    handle_video_provider_result(
        state,
        progress,
        video,
        &context.cfg,
        result,
        VideoProviderResultHandling {
            prompt_id: None,
            completed_cancel_message: None,
            async_job_progress_message: "Seedance 2.0 已提交异步任务，开始等待生成结果",
            missing_result_message: "Seedance 2.0 未返回文件或任务 ID",
        },
    )
    .await
}

fn reference_media_prompt(
    value: &str,
    purpose: VideoProviderRequestPurpose,
    note: &mut Option<String>,
) -> Result<String, String> {
    let prompt = value.trim();
    if prompt.chars().count() >= MIN_PROVIDER_VISUAL_PROMPT_CHARS {
        return Ok(prompt.to_string());
    }
    if purpose == VideoProviderRequestPurpose::Generate {
        Err(format!(
            "参考媒体视频 prompt 过短，至少需要 {} 个字符，并明确写出每个 @image_file_N / @video_file_N / @audio_file_N 的用途",
            MIN_PROVIDER_VISUAL_PROMPT_CHARS
        ))
    } else {
        append_provider_input_note(
            note,
            format!(
                "参考媒体视频 prompt 过短，至少需要 {} 个字符；预览使用占位 prompt 展示请求结构，真实生成仍需补全",
                MIN_PROVIDER_VISUAL_PROMPT_CHARS
            ),
        );
        Ok(if prompt.is_empty() {
            REFERENCE_MEDIA_PREVIEW_PROMPT_PLACEHOLDER.to_string()
        } else {
            format!("{prompt}\n\n{REFERENCE_MEDIA_PREVIEW_PROMPT_PLACEHOLDER}")
        })
    }
}

fn reference_media_provider_request_preview(
    preview_id: &str,
    preview_title: &str,
    source_label: &str,
    context: &ReferenceMediaProviderRequestPreviewContext,
) -> Result<ModelRequestPreview, String> {
    let (request_title, request_kind, note, request) =
        reference_media_provider_preview_payload(source_label, context)?;
    Ok(model_request_preview(
        preview_id,
        preview_title,
        vec![provider_request_step(
            "reference_media_video",
            request_title,
            request_kind,
            Some("reference_media_video".to_string()),
            "ready",
            Some(note),
            &request,
        )?],
    ))
}

fn canvas_video_provider_request_preview(
    context: &CanvasVideoProviderRequestContext,
) -> Result<ModelRequestPreview, String> {
    let request = canvas_video_provider_preview_payload(context)?;
    Ok(model_request_preview(
        "canvas_video_generation",
        "画布视频模型请求",
        vec![provider_request_step(
            "canvas_video",
            "画布视频 provider 请求",
            "video_generation_request",
            Some("canvas_video_prompt".to_string()),
            "ready",
            context.provider_input_note.clone(),
            &request,
        )?],
    ))
}

fn canvas_video_provider_request_snapshot(
    context: &CanvasVideoProviderRequestContext,
) -> Result<Value, String> {
    Ok(provider_request_snapshot_value(
        "canvas_video",
        "画布视频 provider 请求",
        "video_generation_request",
        Some("canvas_video_prompt"),
        context.provider_input_note.clone(),
        canvas_video_provider_preview_payload(context)?,
    ))
}

fn canvas_video_provider_preview_payload(
    context: &CanvasVideoProviderRequestContext,
) -> Result<Value, String> {
    serde_json::to_value(json!({
        "config_id": context.cfg.id,
        "provider_backend": context.cfg.backend,
        "model": context.cfg.model,
        "graph_id": context.graph_id,
        "node_id": context.node_id,
        "request": context.request,
    }))
    .map_err(|err| err.to_string())
}

fn reference_media_provider_request_snapshot(
    context: &ReferenceMediaProviderRequestPreviewContext,
) -> Result<Value, String> {
    let (request_title, request_kind, note, request) =
        reference_media_provider_preview_payload("生成时请求快照", context)?;
    Ok(provider_request_snapshot_value(
        "reference_media_video",
        request_title,
        request_kind,
        Some("reference_media_video"),
        Some(note),
        request,
    ))
}

fn reference_media_provider_preview_payload(
    source_label: &str,
    context: &ReferenceMediaProviderRequestPreviewContext,
) -> Result<(&'static str, &'static str, String, Value), String> {
    let request = Seedance2VideoProvider::request_body_preview(&context.request)
        .map_err(|e| e.to_string())?;
    let reference_media = context
        .request
        .reference_media
        .as_ref()
        .ok_or_else(|| "参考媒体请求缺少 reference_media".to_string())?;
    let config_note = match (context.config_id, context.provider_backend.as_deref()) {
        (Some(config_id), Some(backend)) => format!("config_id={config_id}，backend={backend}；"),
        (Some(config_id), None) => format!("config_id={config_id}；"),
        (None, Some(backend)) => format!("backend={backend}；"),
        (None, None) => String::new(),
    };
    let mut note = format!(
        "{source_label}；{config_note}Seedance 2.0 参考媒体模式使用 functionMode=omni_reference；image_files={}，video_files={}，audio_files={}；prompt 必须用 @image_file_N / @video_file_N / @audio_file_N 明确绑定每个媒体用途。",
        reference_media.image_files.len(),
        reference_media.video_files.len(),
        reference_media.audio_files.len()
    );
    if let Some(input_note) = context.provider_input_note.as_deref() {
        note.push('；');
        note.push_str(input_note);
    }
    Ok((
        "Seedance 2.0 /task/submit 请求",
        "video_generation_seedance2_omni_reference",
        note,
        request,
    ))
}

fn reference_media_urls(
    values: Vec<String>,
    field: &str,
    max_count: usize,
    purpose: VideoProviderRequestPurpose,
    note: &mut Option<String>,
) -> Result<Vec<String>, String> {
    if values.len() > max_count {
        if purpose == VideoProviderRequestPurpose::Generate {
            return Err(format!("{field} 最多允许 {max_count} 个 URL"));
        }
        append_provider_input_note(
            note,
            format!(
                "{field} 选择了 {} 个 URL，超过真实生成上限 {max_count} 个；预览仍展示请求结构，真实生成会失败",
                values.len()
            ),
        );
    }
    let mut normalized = Vec::with_capacity(values.len());
    for (index, value) in values.into_iter().enumerate() {
        let value = value.trim().to_string();
        if value.starts_with("https://") || value.starts_with("http://") {
            normalized.push(value);
            continue;
        }
        if purpose == VideoProviderRequestPurpose::Generate {
            if value.is_empty() {
                return Err(format!("{field}[{index}] 不能为空"));
            }
            return Err(format!(
                "{field}[{index}] 必须以小写 http:// 或 https:// 开头，并且可被 Seedance 2.0 服务访问"
            ));
        }
        let reason = if value.is_empty() {
            "为空"
        } else {
            "未以小写 http:// 或 https:// 开头"
        };
        append_provider_input_note(
            note,
            format!(
                "{field}[{index}] {reason}；预览使用占位 URL 展示请求结构，真实生成仍需填写以小写 http:// 或 https:// 开头且可访问的媒体 URL"
            ),
        );
        normalized.push(reference_media_preview_placeholder_url(field, index));
    }
    Ok(normalized)
}

fn reference_media_preview_placeholder_url(field: &str, index: usize) -> String {
    format!("https://example.invalid/a4n-preview/{field}/{}", index + 1)
}

fn ensure_reference_media_tokens(
    prompt: &mut String,
    token_prefix: &str,
    count: usize,
    purpose: VideoProviderRequestPurpose,
    note: &mut Option<String>,
) -> Result<(), String> {
    let missing = (1..=count)
        .map(|index| format!("@{token_prefix}_{index}"))
        .filter(|token| !prompt.contains(token))
        .collect::<Vec<_>>();
    if missing.is_empty() {
        return Ok(());
    }
    if purpose == VideoProviderRequestPurpose::Generate {
        return Err(format!(
            "参考媒体 prompt 缺少 {} 的用途描述；每个 {token_prefix} 条目都必须在 prompt 中显式引用",
            missing[0]
        ));
    }
    append_provider_input_note(
        note,
        format!(
            "参考媒体 prompt 缺少 {} 的用途描述；预览追加 token 占位说明，真实生成仍需显式引用每个媒体 token",
            missing.join("、")
        ),
    );
    prompt.push_str("\n\n参考媒体绑定占位：\n");
    for token in missing {
        prompt.push_str(&format!(
            "- {token}：请求预览占位用途；真实生成前需要填写该媒体的主体、细节和约束。\n"
        ));
    }
    Ok(())
}

pub(crate) fn reference_media_dimensions_for_aspect_ratio(
    aspect_ratio: &str,
) -> Result<(i32, i32), String> {
    match aspect_ratio.trim() {
        "21:9" => Ok((1680, 720)),
        "16:9" => Ok((1280, 720)),
        "4:3" => Ok((1024, 768)),
        "1:1" => Ok((1024, 1024)),
        "3:4" => Ok((768, 1024)),
        "9:16" => Ok((720, 1280)),
        other => Err(format!(
            "Seedance 2.0 参考媒体模式不支持画幅 {other}，请选择 21:9、16:9、4:3、1:1、3:4 或 9:16"
        )),
    }
}

#[cfg(test)]
mod tests;

pub async fn poll_video_job(
    state: &AppState,
    user_id: i64,
    video_id: i64,
) -> Result<GeneratedVideoSummary, String> {
    let video = generated_video::Entity::find_by_id(video_id)
        .one(state.db.as_ref())
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "视频记录不存在".to_string())?;
    access::verify_project_editor_string(state, user_id, video.project_id).await?;
    job_lifecycle::poll_video_job(state, video_id).await
}
