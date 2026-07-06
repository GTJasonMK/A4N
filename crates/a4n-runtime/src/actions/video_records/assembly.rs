use super::MAX_INLINE_PLAYBACK_VIDEO_BYTES;
use super::imports::{
    ImportedVideoSource, delete_video_record_error, import_video_output_path,
    mark_imported_video_completed, probe_video_properties, uploaded_video_source,
};
use crate::AppState;
use crate::actions::access;
use crate::actions::video_generation::media::{
    append_secondary_error, best_effort_video_thumbnail_path, ensure_chapter_videos_dir,
    ensure_ffmpeg, remove_partial_video_file, video_mime_from_path,
};
use crate::actions::video_generation::{GeneratedVideoSummary, video_to_summary};
use crate::actions::video_prompts::resolve_project_chapter_video_source;
use crate::file_utils::{canonicalize_data_file_path, data_dir_relative_file_path};
use a4n_core::models::ai::generated_video;
use a4n_services::video::{CreateVideoRecord, UpdateVideoRecord};
use base64::Engine as _;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::UNIX_EPOCH;

const CHAPTER_VIDEO_TYPE: &str = "chapter_video";
pub(super) const ASSEMBLY_INSERT_VIDEO_TYPE: &str = "assembly_insert";
const ASSEMBLY_PUSH_VIDEO_TYPE: &str = "assembly_push";
const ASSEMBLY_POSTPROCESS_EDIT_VIDEO_TYPE: &str = "assembly_postprocess_edit";
const ASSEMBLY_ORDER_STEP: f64 = 1000.0;
const ASSEMBLY_PUSH_STATUS_PENDING: &str = "pending";
const ASSEMBLY_PUSH_STATUS_ACCEPTED: &str = "accepted";
const ASSEMBLY_PUSH_STATUS_REJECTED: &str = "rejected";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssembledChapterVideoClipSummary {
    pub video_id: i64,
    pub video_type: String,
    pub shot_number: i32,
    pub sort_order: f64,
    pub insert_after_video_id: Option<i64>,
    pub duration_secs: f32,
    pub file_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssembledChapterVideoPreview {
    pub mime_type: String,
    pub data_base64: String,
    pub file_path: String,
    pub video_ids: Vec<i64>,
    pub clips: Vec<AssembledChapterVideoClipSummary>,
    pub duration_secs: f32,
    pub file_size: i64,
    pub updated_at: Option<String>,
    pub cached: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct AssemblyPushResult {
    pub video: GeneratedVideoSummary,
    pub source_video_ids: Vec<i64>,
    pub push_index: i32,
    pub status: String,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct PushAssembledChapterVideoInput {
    #[serde(default)]
    pub end_video_id: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpdateAssemblyPushStatusInput {
    pub action: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ApplyAssemblyPostprocessEditInput {
    pub operation: String,
    #[serde(default)]
    pub at_secs: Option<f32>,
    #[serde(default)]
    pub start_secs: Option<f32>,
    #[serde(default)]
    pub end_secs: Option<f32>,
    #[serde(default)]
    pub source_video_id: Option<i64>,
    #[serde(default)]
    pub source_file_name: Option<String>,
    #[serde(default)]
    pub source_mime_type: Option<String>,
    #[serde(default)]
    pub source_data_base64: Option<String>,
}

#[derive(Debug, Clone)]
pub(super) struct AssemblyClip {
    pub(super) video_id: i64,
    pub(super) video_type: String,
    pub(super) shot_number: i32,
    pub(super) sort_order: f64,
    pub(super) insert_after_video_id: Option<i64>,
    pub(super) path: PathBuf,
    pub(super) file_path: String,
    pub(super) duration_secs: f32,
    pub(super) width: i32,
    pub(super) height: i32,
    pub(super) fps: i32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(super) struct AssemblyCacheManifest {
    pub(super) version: u32,
    pub(super) project_id: i64,
    pub(super) chapter_id: i64,
    pub(super) workflow_mode: String,
    pub(super) width: i32,
    pub(super) height: i32,
    pub(super) fps: i32,
    pub(super) audio_enabled: bool,
    pub(super) clips: Vec<AssemblyCacheClip>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(super) struct AssemblyCacheClip {
    pub(super) video_id: i64,
    pub(super) video_type: String,
    pub(super) shot_number: i32,
    pub(super) sort_order: f64,
    pub(super) insert_after_video_id: Option<i64>,
    pub(super) file_path: String,
    pub(super) file_size: u64,
    pub(super) modified_unix_millis: u64,
    pub(super) has_audio: bool,
    pub(super) duration_secs: f32,
    pub(super) width: i32,
    pub(super) height: i32,
    pub(super) fps: i32,
}

struct AssemblyRequestContext {
    clips: Vec<AssemblyClip>,
    output_path: PathBuf,
    manifest_path: PathBuf,
    manifest: AssemblyCacheManifest,
}

#[derive(Debug)]
pub(super) struct AssemblyPushRow {
    pub(super) video: generated_video::Model,
    pub(super) status: String,
    pub(super) push_index: i32,
    pub(super) source_video_ids: Vec<i64>,
}

#[derive(Debug, Clone)]
struct AssemblyInsertMetadata {
    sort_order: f64,
    insert_after_video_id: Option<i64>,
    insert_after_shot_number: Option<i32>,
}

#[derive(Debug, Clone)]
pub(super) struct PostprocessEditMetadata {
    pub(super) push_video_id: i64,
    pub(super) base_video_id: i64,
}

pub(super) struct PostprocessEditSource {
    path: PathBuf,
    file_name: Option<String>,
    source_video_id: Option<i64>,
    _uploaded_source: Option<ImportedVideoSource>,
}

impl AssemblyRequestContext {
    fn cache_matches(&self) -> Result<bool, String> {
        assembly_cache_matches(&self.output_path, &self.manifest_path, &self.manifest)
    }

    fn preview(
        &self,
        data_dir: &Path,
        cached: bool,
    ) -> Result<AssembledChapterVideoPreview, String> {
        read_assembled_video_preview(data_dir, &self.output_path, &self.clips, cached)
    }
}

pub async fn assemble_chapter_videos(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    chapter_id: i64,
    workflow_mode: String,
) -> Result<AssembledChapterVideoPreview, String> {
    let context = assembly_request_context(
        state,
        user_id,
        project_id,
        chapter_id,
        workflow_mode,
        "拼接成片",
    )
    .await?;
    if context.clips.is_empty() {
        return Err("当前视频生成流程还没有可拼接的已完成视频".to_string());
    }

    if context.cache_matches()? {
        return context.preview(&state.config.data_dir, true);
    }

    run_ffmpeg_assemble_videos(
        &context.clips,
        &context.output_path,
        context.manifest.width,
        context.manifest.height,
        context.manifest.fps,
        &context.manifest,
    )?;
    write_assembly_cache_manifest(&context.manifest_path, &context.manifest)?;

    context.preview(&state.config.data_dir, false)
}

pub async fn load_cached_assembled_chapter_video_preview(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    chapter_id: i64,
    workflow_mode: String,
) -> Result<Option<AssembledChapterVideoPreview>, String> {
    let context = assembly_request_context(
        state,
        user_id,
        project_id,
        chapter_id,
        workflow_mode,
        "加载成片缓存",
    )
    .await?;
    if context.clips.is_empty() {
        return Ok(None);
    }
    if !context.cache_matches()? {
        return Ok(None);
    }
    context.preview(&state.config.data_dir, true).map(Some)
}

fn read_assembled_video_preview(
    data_dir: &Path,
    output_path: &Path,
    clips: &[AssemblyClip],
    cached: bool,
) -> Result<AssembledChapterVideoPreview, String> {
    let metadata =
        std::fs::metadata(output_path).map_err(|e| format!("无法读取成片文件元数据：{e}"))?;
    if metadata.len() > MAX_INLINE_PLAYBACK_VIDEO_BYTES {
        return Err(format!(
            "成片文件超过 {}MB，无法以内置播放器 Blob 方式加载",
            MAX_INLINE_PLAYBACK_VIDEO_BYTES / 1024 / 1024
        ));
    }
    let bytes = std::fs::read(output_path).map_err(|e| format!("无法读取成片文件：{e}"))?;
    let duration_secs = clips.iter().map(|clip| clip.duration_secs.max(0.0)).sum();
    let video_ids = clips.iter().map(|clip| clip.video_id).collect();
    let clip_summaries = clips
        .iter()
        .map(|clip| AssembledChapterVideoClipSummary {
            video_id: clip.video_id,
            video_type: clip.video_type.clone(),
            shot_number: clip.shot_number,
            sort_order: clip.sort_order,
            insert_after_video_id: clip.insert_after_video_id,
            duration_secs: clip.duration_secs,
            file_path: clip.file_path.clone(),
        })
        .collect();
    Ok(AssembledChapterVideoPreview {
        mime_type: video_mime_from_path(output_path)?.to_string(),
        data_base64: base64::engine::general_purpose::STANDARD.encode(bytes),
        file_path: data_dir_relative_file_path(data_dir, output_path, "成片文件")?,
        video_ids,
        clips: clip_summaries,
        duration_secs,
        file_size: i64::try_from(metadata.len())
            .map_err(|_| "成片文件过大，无法记录文件大小".to_string())?,
        updated_at: Some(Utc::now().to_rfc3339()),
        cached,
    })
}

pub async fn push_assembled_chapter_video(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    chapter_id: i64,
    workflow_mode: String,
    input: PushAssembledChapterVideoInput,
) -> Result<AssemblyPushResult, String> {
    let context = assembly_request_context(
        state,
        user_id,
        project_id,
        chapter_id,
        workflow_mode,
        "推送成片片段",
    )
    .await?;
    if context.clips.is_empty() {
        return Err("当前视频生成流程还没有可推送的成片片段".to_string());
    }
    let rows = state
        .video
        .list_by_chapter(project_id, chapter_id)
        .await
        .map_err(|e| e.to_string())?;
    let pushes = assembly_push_rows(&rows, context.manifest.workflow_mode.as_str())?;
    if pushes
        .iter()
        .any(|push| push.status == ASSEMBLY_PUSH_STATUS_PENDING)
    {
        return Err("当前最新推送尚未同意或拒绝，不能继续推送后续片段".to_string());
    }

    let active_source_ids = active_assembly_push_source_ids(&pushes);
    ensure_active_pushes_match_clip_prefix(&context.clips, &active_source_ids)?;
    let pending_clips = &context.clips[active_source_ids.len()..];
    if pending_clips.is_empty() {
        return Err("没有新的后续成片片段可推送".to_string());
    }
    let source_clips = selected_assembly_push_source_clips(pending_clips, input.end_video_id)?;
    let push_index = pushes
        .iter()
        .map(|push| push.push_index)
        .max()
        .unwrap_or(0)
        .saturating_add(1);
    let duration_secs = source_clips
        .iter()
        .map(|clip| clip.duration_secs.max(0.0))
        .sum::<f32>();
    let source_video_ids = source_clips
        .iter()
        .map(|clip| clip.video_id)
        .collect::<Vec<_>>();
    let source_sort_orders = source_clips
        .iter()
        .map(|clip| clip.sort_order)
        .collect::<Vec<_>>();
    let (width, height) = assembly_dimensions(&source_clips);
    let fps = assembly_fps(&source_clips);
    let metadata = json!({
        "assembly_push": {
            "status": ASSEMBLY_PUSH_STATUS_PENDING,
            "push_index": push_index,
            "workflow_mode": context.manifest.workflow_mode,
            "source_video_ids": source_video_ids,
            "source_sort_orders": source_sort_orders,
            "source_clip_count": source_clips.len(),
            "pushed_at": Utc::now().to_rfc3339(),
        }
    });
    let record = state
        .video
        .create_video_record(CreateVideoRecord {
            project_id,
            chapter_id: Some(chapter_id),
            video_type: ASSEMBLY_PUSH_VIDEO_TYPE.to_string(),
            workflow_mode: Some(context.manifest.workflow_mode.clone()),
            prompt: format!("成片后处理推送批次 {push_index}"),
            negative_prompt: None,
            model: "a4n-assembly-push".to_string(),
            width,
            height,
            duration_secs,
            fps,
            source_video_id: source_video_ids.first().copied(),
            source_image_id: None,
            provider_job_id: None,
            metadata_json: Some(metadata),
        })
        .await
        .map_err(|e| e.to_string())?;
    let output = import_video_output_path(
        state,
        project_id,
        chapter_id,
        record.id,
        "assembly_push",
        "mp4",
    )?;
    let manifest = build_assembly_cache_manifest(
        project_id,
        chapter_id,
        &context.manifest.workflow_mode,
        width,
        height,
        fps,
        &source_clips,
    )?;
    if let Err(err) =
        run_ffmpeg_assemble_videos(&source_clips, &output, width, height, fps, &manifest)
    {
        let delete_error = delete_video_record_error(state, record.id).await;
        return Err(append_secondary_error(err, delete_error));
    }
    let output_path_str = data_dir_relative_file_path(&state.config.data_dir, &output, "推送成片")?;
    let thumbnail_path = best_effort_video_thumbnail_path(
        &state.config.data_dir,
        project_id,
        chapter_id,
        record.id,
        &output,
    );
    let size = std::fs::metadata(&output)
        .map(|m| m.len() as i64)
        .map_err(|e| format!("无法读取推送成片文件元数据：{e}"))?;
    let updated = mark_imported_video_completed(
        state,
        record.id,
        &output,
        &output_path_str,
        thumbnail_path.as_deref(),
        size,
        None,
        None,
        None,
        "推送成片记录完成失败",
        "推送成片文件",
        "推送成片缩略图",
    )
    .await?;
    Ok(AssemblyPushResult {
        video: video_to_summary(&updated),
        source_video_ids,
        push_index,
        status: ASSEMBLY_PUSH_STATUS_PENDING.to_string(),
    })
}

pub async fn update_assembly_push_status(
    state: &AppState,
    user_id: i64,
    video_id: i64,
    input: UpdateAssemblyPushStatusInput,
) -> Result<GeneratedVideoSummary, String> {
    let existing = state
        .video
        .get_video(video_id)
        .await
        .map_err(|e| e.to_string())?;
    access::verify_project_editor_string(state, user_id, existing.project_id).await?;
    if existing.video_type != ASSEMBLY_PUSH_VIDEO_TYPE {
        return Err(format!("视频 #{} 不是成片后处理推送批次", existing.id));
    }
    let chapter_id = existing
        .chapter_id
        .ok_or_else(|| format!("推送批次 #{} 缺少章节绑定", existing.id))?;
    let workflow_mode = existing
        .workflow_mode
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("推送批次 #{} 缺少 workflow_mode", existing.id))?;
    let rows = state
        .video
        .list_by_chapter(existing.project_id, chapter_id)
        .await
        .map_err(|e| e.to_string())?;
    let pushes = assembly_push_rows(&rows, workflow_mode)?;
    let latest = latest_non_rejected_push(&pushes)
        .ok_or_else(|| "当前章节没有可管理的推送批次".to_string())?;
    if latest.video.id != existing.id {
        return Err("只能管理当前最新推送批次，不能跨推送操作".to_string());
    }
    let action = input.action.trim();
    let next_status = match (action, latest.status.as_str()) {
        ("accept", ASSEMBLY_PUSH_STATUS_PENDING) => ASSEMBLY_PUSH_STATUS_ACCEPTED,
        ("reject", ASSEMBLY_PUSH_STATUS_PENDING) => ASSEMBLY_PUSH_STATUS_REJECTED,
        ("undo", ASSEMBLY_PUSH_STATUS_ACCEPTED) => ASSEMBLY_PUSH_STATUS_PENDING,
        ("accept", _) => return Err("只有待处理推送可以同意".to_string()),
        ("reject", _) => return Err("只有待处理推送可以拒绝".to_string()),
        ("undo", _) => return Err("只有已同意的最新推送可以撤销".to_string()),
        _ => return Err(format!("未知推送管理操作：{action}")),
    };
    let metadata = assembly_push_metadata_with_status(existing.metadata_json.clone(), next_status);
    let row = state
        .video
        .update_record(
            existing.id,
            UpdateVideoRecord {
                prompt: None,
                negative_prompt: None,
                clear_negative_prompt: false,
                metadata_json: Some(metadata),
                clear_metadata_json: false,
                is_favorite: None,
            },
        )
        .await
        .map_err(|e| e.to_string())?;
    Ok(video_to_summary(&row))
}

pub async fn apply_assembly_postprocess_edit(
    state: &AppState,
    user_id: i64,
    push_video_id: i64,
    input: ApplyAssemblyPostprocessEditInput,
) -> Result<GeneratedVideoSummary, String> {
    let push = state
        .video
        .get_video(push_video_id)
        .await
        .map_err(|e| e.to_string())?;
    access::verify_project_editor_string(state, user_id, push.project_id).await?;
    if push.video_type != ASSEMBLY_PUSH_VIDEO_TYPE {
        return Err(format!("视频 #{} 不是后处理推送批次", push.id));
    }
    if assembly_push_status(&push)? == ASSEMBLY_PUSH_STATUS_REJECTED {
        return Err("已拒绝的推送批次不能继续编辑".to_string());
    }
    let chapter_id = push
        .chapter_id
        .ok_or_else(|| format!("推送批次 #{} 缺少章节绑定", push.id))?;
    let workflow_mode = push
        .workflow_mode
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("推送批次 #{} 缺少 workflow_mode", push.id))?;
    let rows = state
        .video
        .list_by_chapter(push.project_id, chapter_id)
        .await
        .map_err(|e| e.to_string())?;
    let pushes = assembly_push_rows(&rows, workflow_mode)?;
    let latest = latest_non_rejected_push(&pushes)
        .ok_or_else(|| "当前章节没有可编辑的推送批次".to_string())?;
    if latest.video.id != push.id {
        return Err("只能编辑当前最新推送批次，不能跨推送操作".to_string());
    }
    let base = latest_postprocess_version(state, &push).await?;
    let base_path = base
        .file_path
        .as_deref()
        .map(str::trim)
        .filter(|path| !path.is_empty())
        .ok_or_else(|| format!("后处理基底视频 #{} 没有本地文件", base.id))?;
    let base_path = canonicalize_data_file_path(
        &state.config.data_dir,
        base_path,
        &format!("后处理基底视频 #{}", base.id),
    )?;
    let operation = validate_postprocess_operation(&input.operation)?;
    let source =
        postprocess_edit_source(state, user_id, push.project_id, &input, operation).await?;
    let (start_secs, end_secs, source_duration) =
        postprocess_edit_range(&base, &input, operation, source.as_ref())?;
    let output_duration = match operation {
        "insert" => base.duration_secs.max(0.0) + source_duration.unwrap_or(0.0),
        "delete" => (base.duration_secs - (end_secs - start_secs)).max(0.1),
        "replace" => {
            (base.duration_secs - (end_secs - start_secs) + source_duration.unwrap_or(0.0)).max(0.1)
        }
        _ => base.duration_secs,
    };
    let metadata = json!({
        "postprocess_edit": {
            "push_video_id": push.id,
            "base_video_id": base.id,
            "operation": operation,
            "at_secs": input.at_secs,
            "start_secs": input.start_secs,
            "end_secs": input.end_secs,
            "source_video_id": source.as_ref().and_then(|source| source.source_video_id),
            "source_file_name": source.as_ref().and_then(|source| source.file_name.clone()),
            "edited_at": Utc::now().to_rfc3339(),
        }
    });
    let record = state
        .video
        .create_video_record(CreateVideoRecord {
            project_id: push.project_id,
            chapter_id: Some(chapter_id),
            video_type: ASSEMBLY_POSTPROCESS_EDIT_VIDEO_TYPE.to_string(),
            workflow_mode: push.workflow_mode.clone(),
            prompt: format!("后处理编辑：推送批次 #{} {}", push.id, operation),
            negative_prompt: None,
            model: "a4n-postprocess-edit".to_string(),
            width: base.width,
            height: base.height,
            duration_secs: output_duration,
            fps: base.fps,
            source_video_id: Some(base.id),
            source_image_id: None,
            provider_job_id: None,
            metadata_json: Some(metadata),
        })
        .await
        .map_err(|e| e.to_string())?;
    let output = import_video_output_path(
        state,
        push.project_id,
        chapter_id,
        record.id,
        "postprocess_edit",
        "mp4",
    )?;
    if let Err(err) = run_ffmpeg_postprocess_edit(FfmpegPostprocessEditInput {
        base_path: &base_path,
        source_path: source.as_ref().map(|source| source.path.as_path()),
        output_path: &output,
        operation,
        start_secs,
        end_secs,
        base_duration_secs: base.duration_secs,
        width: base.width,
        height: base.height,
        fps: base.fps,
    }) {
        let delete_error = delete_video_record_error(state, record.id).await;
        return Err(append_secondary_error(err, delete_error));
    }
    let output_path_str =
        data_dir_relative_file_path(&state.config.data_dir, &output, "后处理编辑视频")?;
    let thumbnail_path = best_effort_video_thumbnail_path(
        &state.config.data_dir,
        push.project_id,
        chapter_id,
        record.id,
        &output,
    );
    let size = std::fs::metadata(&output)
        .map(|m| m.len() as i64)
        .map_err(|e| format!("无法读取后处理编辑视频文件元数据：{e}"))?;
    let updated = mark_imported_video_completed(
        state,
        record.id,
        &output,
        &output_path_str,
        thumbnail_path.as_deref(),
        size,
        None,
        None,
        None,
        "后处理编辑视频记录完成失败",
        "后处理编辑视频文件",
        "后处理编辑视频缩略图",
    )
    .await?;
    Ok(video_to_summary(&updated))
}

async fn assembly_request_context(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    chapter_id: i64,
    workflow_mode: String,
    action: &str,
) -> Result<AssemblyRequestContext, String> {
    access::verify_project_editor_string(state, user_id, project_id).await?;
    resolve_project_chapter_video_source(state, project_id, chapter_id, action, false).await?;
    let workflow_mode = validate_assembly_workflow_mode(&workflow_mode)?;
    let rows = state
        .video
        .list_by_chapter(project_id, chapter_id)
        .await
        .map_err(|e| e.to_string())?;
    let clips = assembly_clips(state, rows, project_id, chapter_id, workflow_mode)?;
    let (width, height) = assembly_dimensions(&clips);
    let fps = assembly_fps(&clips);
    let output_path = assembled_video_output_path(state, project_id, chapter_id, workflow_mode)?;
    let manifest_path = assembly_cache_manifest_path(&output_path);
    let manifest = build_assembly_cache_manifest(
        project_id,
        chapter_id,
        workflow_mode,
        width,
        height,
        fps,
        &clips,
    )?;
    Ok(AssemblyRequestContext {
        clips,
        output_path,
        manifest_path,
        manifest,
    })
}

pub(super) fn assembly_push_rows(
    rows: &[generated_video::Model],
    workflow_mode: &str,
) -> Result<Vec<AssemblyPushRow>, String> {
    let mut pushes = rows
        .iter()
        .filter(|row| {
            row.video_type == ASSEMBLY_PUSH_VIDEO_TYPE
                && row.workflow_mode.as_deref() == Some(workflow_mode)
                && row.status == "completed"
        })
        .map(parse_assembly_push_row)
        .collect::<Result<Vec<_>, _>>()?;
    pushes.sort_by(|left, right| {
        left.push_index
            .cmp(&right.push_index)
            .then_with(|| left.video.id.cmp(&right.video.id))
    });
    Ok(pushes)
}

fn parse_assembly_push_row(row: &generated_video::Model) -> Result<AssemblyPushRow, String> {
    let metadata = row
        .metadata_json
        .as_ref()
        .and_then(|metadata| metadata.get("assembly_push"))
        .and_then(Value::as_object)
        .ok_or_else(|| format!("推送批次 #{} 缺少 assembly_push metadata", row.id))?;
    let push_index = metadata
        .get("push_index")
        .and_then(Value::as_i64)
        .and_then(|value| i32::try_from(value).ok())
        .filter(|value| *value > 0)
        .ok_or_else(|| format!("推送批次 #{} 缺少有效 push_index", row.id))?;
    let status = assembly_push_metadata_status(row.id, metadata)?;
    let values = metadata
        .get("source_video_ids")
        .and_then(Value::as_array)
        .ok_or_else(|| format!("推送批次 #{} 缺少 source_video_ids", row.id))?;
    if values.is_empty() {
        return Err(format!("推送批次 #{} 的 source_video_ids 不能为空", row.id));
    }
    let mut seen = HashSet::new();
    let source_video_ids = values
        .iter()
        .enumerate()
        .map(|(index, value)| {
            let id = value.as_i64().filter(|id| *id > 0).ok_or_else(|| {
                format!(
                    "推送批次 #{} 的 source_video_ids[{index}] 必须是正整数",
                    row.id
                )
            })?;
            if !seen.insert(id) {
                return Err(format!(
                    "推送批次 #{} 的 source_video_ids 包含重复 ID：{id}",
                    row.id
                ));
            }
            Ok(id)
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(AssemblyPushRow {
        video: row.clone(),
        status,
        push_index,
        source_video_ids,
    })
}

fn assembly_push_metadata_status(
    video_id: i64,
    metadata: &serde_json::Map<String, Value>,
) -> Result<String, String> {
    let status = metadata
        .get("status")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("推送批次 #{video_id} 缺少 assembly_push.status"))?;
    match status {
        ASSEMBLY_PUSH_STATUS_PENDING
        | ASSEMBLY_PUSH_STATUS_ACCEPTED
        | ASSEMBLY_PUSH_STATUS_REJECTED => Ok(status.to_string()),
        other => Err(format!(
            "推送批次 #{video_id} 的 assembly_push.status 无效: {other}"
        )),
    }
}

pub(super) fn active_assembly_push_source_ids(pushes: &[AssemblyPushRow]) -> Vec<i64> {
    pushes
        .iter()
        .filter(|push| push.status != ASSEMBLY_PUSH_STATUS_REJECTED)
        .flat_map(|push| push.source_video_ids.iter().copied())
        .collect()
}

pub(super) fn selected_assembly_push_source_clips(
    pending_clips: &[AssemblyClip],
    end_video_id: Option<i64>,
) -> Result<Vec<AssemblyClip>, String> {
    if pending_clips.is_empty() {
        return Err("没有新的后续成片片段可推送".to_string());
    }
    if let Some(end_video_id) = end_video_id {
        let end_index = pending_clips
            .iter()
            .position(|clip| clip.video_id == end_video_id)
            .ok_or_else(|| format!("推送结束片段 #{end_video_id} 不在可推送后续片段中"))?;
        return Ok(pending_clips[..=end_index].to_vec());
    }
    Ok(pending_clips.to_vec())
}

fn latest_non_rejected_push(pushes: &[AssemblyPushRow]) -> Option<&AssemblyPushRow> {
    pushes
        .iter()
        .rev()
        .find(|push| push.status != ASSEMBLY_PUSH_STATUS_REJECTED)
}

pub(super) fn ensure_active_pushes_match_clip_prefix(
    clips: &[AssemblyClip],
    active_source_ids: &[i64],
) -> Result<(), String> {
    if active_source_ids.len() > clips.len() {
        return Err("已推送片段数量超过当前成片片段数量，请检查成片列表".to_string());
    }
    for (index, source_id) in active_source_ids.iter().enumerate() {
        let clip = &clips[index];
        if clip.video_id != *source_id {
            return Err(format!(
                "已推送片段与当前成片列表不连续：第 {} 个已推送源片段应为 #{}，当前为 #{}",
                index + 1,
                source_id,
                clip.video_id
            ));
        }
    }
    Ok(())
}

fn assembly_push_status(video: &generated_video::Model) -> Result<String, String> {
    let status = video
        .metadata_json
        .as_ref()
        .and_then(|metadata| metadata.get("assembly_push"))
        .and_then(|metadata| metadata.get("status"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("推送批次 #{} 缺少 assembly_push.status", video.id))?;
    match status {
        ASSEMBLY_PUSH_STATUS_PENDING
        | ASSEMBLY_PUSH_STATUS_ACCEPTED
        | ASSEMBLY_PUSH_STATUS_REJECTED => Ok(status.to_string()),
        other => Err(format!(
            "推送批次 #{} 的 assembly_push.status 无效: {other}",
            video.id
        )),
    }
}

fn assembly_push_metadata_with_status(base_metadata: Option<Value>, status: &str) -> Value {
    let mut metadata = match base_metadata {
        Some(Value::Object(map)) => Value::Object(map),
        Some(value) => json!({ "metadata": value }),
        None => json!({}),
    };
    let Some(map) = metadata.as_object_mut() else {
        return json!({ "assembly_push": { "status": status } });
    };
    let entry = map
        .entry("assembly_push".to_string())
        .or_insert_with(|| json!({}));
    if !entry.is_object() {
        *entry = json!({});
    }
    if let Some(push) = entry.as_object_mut() {
        push.insert("status".to_string(), json!(status));
        push.insert(
            "status_updated_at".to_string(),
            json!(Utc::now().to_rfc3339()),
        );
    }
    metadata
}

pub(super) async fn pushed_source_ids_for_chapter(
    state: &AppState,
    project_id: i64,
    chapter_id: i64,
    workflow_mode: &str,
) -> Result<HashSet<i64>, String> {
    let rows = state
        .video
        .list_by_chapter(project_id, chapter_id)
        .await
        .map_err(|e| e.to_string())?;
    Ok(
        active_assembly_push_source_ids(&assembly_push_rows(&rows, workflow_mode)?)
            .into_iter()
            .collect(),
    )
}

pub(super) fn ensure_insert_position_not_inside_pushed_prefix(
    clips: &[AssemblyClip],
    pushes: &[AssemblyPushRow],
    insert_after_video_id: Option<i64>,
) -> Result<(), String> {
    let active_source_ids = active_assembly_push_source_ids(pushes);
    ensure_active_pushes_match_clip_prefix(clips, &active_source_ids)?;
    if active_source_ids.is_empty() {
        return Ok(());
    }
    let Some(anchor_id) = insert_after_video_id else {
        return Err("已有成片片段推送后，不能再向已推送列表开头插入衔接片段".to_string());
    };
    let anchor_index = clips
        .iter()
        .position(|clip| clip.video_id == anchor_id)
        .ok_or_else(|| format!("插入位置视频 #{anchor_id} 不在当前成片片段中"))?;
    let pushed_last_index = active_source_ids.len().saturating_sub(1);
    if anchor_index < pushed_last_index {
        return Err("不能在已推送成片片段中间插入衔接片段".to_string());
    }
    Ok(())
}

pub(super) fn validate_assembly_workflow_mode(value: &str) -> Result<&'static str, String> {
    match value.trim() {
        "boundary_frames" => Ok("boundary_frames"),
        "first_frame_chain" => Ok("first_frame_chain"),
        "reference_media" => Ok("reference_media"),
        "chat_video" => Ok("chat_video"),
        "" => Err("请选择视频生成流程后再拼接成片".to_string()),
        other => Err(format!("视频生成流程无效：{other}")),
    }
}

pub(super) fn assembly_clips(
    state: &AppState,
    rows: Vec<generated_video::Model>,
    project_id: i64,
    chapter_id: i64,
    workflow_mode: &str,
) -> Result<Vec<AssemblyClip>, String> {
    let mut clips = Vec::new();
    for video in rows {
        if video.project_id != project_id
            || video.chapter_id != Some(chapter_id)
            || video.workflow_mode.as_deref() != Some(workflow_mode)
            || !is_assembly_clip_video_type(&video.video_type)
            || video.status != "completed"
        {
            continue;
        }
        let Some(file_path) = video
            .file_path
            .as_deref()
            .map(str::trim)
            .filter(|path| !path.is_empty())
        else {
            continue;
        };
        let shot_number = assembly_clip_display_shot_number(&video)?;
        let sort_order = assembly_clip_sort_order(&video)?;
        let insert_after_video_id = assembly_clip_insert_after_video_id(&video)?;
        let path = canonicalize_data_file_path(
            &state.config.data_dir,
            file_path,
            &format!("成片片段视频 #{}", video.id),
        )?;
        clips.push(AssemblyClip {
            video_id: video.id,
            video_type: video.video_type,
            shot_number,
            sort_order,
            insert_after_video_id,
            path,
            file_path: file_path.to_string(),
            duration_secs: video.duration_secs,
            width: video.width,
            height: video.height,
            fps: video.fps,
        });
    }
    clips.sort_by(|left, right| {
        left.sort_order
            .total_cmp(&right.sort_order)
            .then_with(|| left.video_id.cmp(&right.video_id))
    });
    Ok(clips)
}

pub(super) fn is_assembly_clip_video_type(video_type: &str) -> bool {
    matches!(video_type, CHAPTER_VIDEO_TYPE | ASSEMBLY_INSERT_VIDEO_TYPE)
}

fn assembly_clip_display_shot_number(video: &generated_video::Model) -> Result<i32, String> {
    match video.video_type.as_str() {
        CHAPTER_VIDEO_TYPE => assembly_clip_shot_number(video),
        ASSEMBLY_INSERT_VIDEO_TYPE => Ok(parse_assembly_insert_metadata(video)?
            .insert_after_shot_number
            .unwrap_or(0)),
        _ => Err(format!("视频 #{} 不是成片片段", video.id)),
    }
}

pub(super) fn assembly_clip_sort_order(video: &generated_video::Model) -> Result<f64, String> {
    match video.video_type.as_str() {
        CHAPTER_VIDEO_TYPE => {
            Ok(f64::from(assembly_clip_shot_number(video)?) * ASSEMBLY_ORDER_STEP)
        }
        ASSEMBLY_INSERT_VIDEO_TYPE => assembly_insert_sort_order(video),
        _ => Err(format!("视频 #{} 不是成片片段", video.id)),
    }
}

pub(super) fn assembly_clip_shot_number(video: &generated_video::Model) -> Result<i32, String> {
    video
        .metadata_json
        .as_ref()
        .and_then(|metadata| metadata.get("shot_number"))
        .and_then(|value| value.as_i64())
        .and_then(|value| i32::try_from(value).ok())
        .filter(|value| *value > 0)
        .ok_or_else(|| {
            format!(
                "视频 #{} 缺少有效 shot_number，无法确定成片拼接顺序",
                video.id
            )
        })
}

fn assembly_insert_sort_order(video: &generated_video::Model) -> Result<f64, String> {
    Ok(parse_assembly_insert_metadata(video)?.sort_order)
}

fn assembly_clip_insert_after_video_id(
    video: &generated_video::Model,
) -> Result<Option<i64>, String> {
    match video.video_type.as_str() {
        CHAPTER_VIDEO_TYPE => Ok(None),
        ASSEMBLY_INSERT_VIDEO_TYPE => {
            Ok(parse_assembly_insert_metadata(video)?.insert_after_video_id)
        }
        _ => Err(format!("视频 #{} 不是成片片段", video.id)),
    }
}

fn parse_assembly_insert_metadata(
    video: &generated_video::Model,
) -> Result<AssemblyInsertMetadata, String> {
    let metadata = video
        .metadata_json
        .as_ref()
        .and_then(|metadata| metadata.get("assembly_insert"))
        .and_then(Value::as_object)
        .ok_or_else(|| format!("插入片段 #{} 缺少 assembly_insert metadata", video.id))?;
    let sort_order = metadata
        .get("sort_order")
        .and_then(Value::as_f64)
        .filter(|value| value.is_finite())
        .ok_or_else(|| {
            format!(
                "插入片段 #{} 缺少有效 sort_order，无法确定成片拼接顺序",
                video.id
            )
        })?;
    let insert_after_video_id = optional_positive_i64(metadata.get("insert_after_video_id"))
        .map_err(|err| {
            format!(
                "插入片段 #{} 的 assembly_insert.insert_after_video_id {err}",
                video.id
            )
        })?;
    let insert_after_shot_number = optional_positive_i32(metadata.get("insert_after_shot_number"))
        .map_err(|err| {
            format!(
                "插入片段 #{} 的 assembly_insert.insert_after_shot_number {err}",
                video.id
            )
        })?;
    Ok(AssemblyInsertMetadata {
        sort_order,
        insert_after_video_id,
        insert_after_shot_number,
    })
}

fn optional_positive_i64(value: Option<&Value>) -> Result<Option<i64>, String> {
    match value {
        None | Some(Value::Null) => Ok(None),
        Some(value) => value
            .as_i64()
            .filter(|id| *id > 0)
            .map(Some)
            .ok_or_else(|| "必须为空或正整数".to_string()),
    }
}

fn optional_positive_i32(value: Option<&Value>) -> Result<Option<i32>, String> {
    match optional_positive_i64(value)? {
        Some(value) => i32::try_from(value)
            .map(Some)
            .map_err(|_| "超过可记录范围".to_string()),
        None => Ok(None),
    }
}

pub(super) async fn resolve_assembly_insert_position_metadata(
    state: &AppState,
    video: &generated_video::Model,
    base_metadata: Option<Value>,
    insert_after_video_id: Option<i64>,
) -> Result<Value, String> {
    if video.video_type != ASSEMBLY_INSERT_VIDEO_TYPE {
        return Err(format!(
            "视频 #{} 不是成片插入片段，不能调整插入位置",
            video.id
        ));
    }
    if insert_after_video_id == Some(video.id) {
        return Err("插入片段不能插入到自己之后".to_string());
    }
    let chapter_id = video
        .chapter_id
        .ok_or_else(|| format!("插入片段 #{} 缺少章节绑定", video.id))?;
    let workflow_mode = video
        .workflow_mode
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("插入片段 #{} 缺少 workflow_mode", video.id))?;
    let rows = state
        .video
        .list_by_chapter(video.project_id, chapter_id)
        .await
        .map_err(|e| e.to_string())?
        .into_iter()
        .filter(|row| row.id != video.id)
        .collect::<Vec<_>>();
    let clips = assembly_clips(state, rows, video.project_id, chapter_id, workflow_mode)?;
    let (sort_order, insert_after_shot_number) =
        next_assembly_insert_sort_order(&clips, insert_after_video_id)?;
    assembly_insert_metadata_with_position(
        base_metadata,
        sort_order,
        insert_after_video_id,
        insert_after_shot_number,
    )
}

pub(super) fn assembly_insert_metadata_with_position(
    base_metadata: Option<Value>,
    sort_order: f64,
    insert_after_video_id: Option<i64>,
    insert_after_shot_number: Option<i32>,
) -> Result<Value, String> {
    let mut metadata = match base_metadata {
        Some(Value::Object(map)) => Value::Object(map),
        Some(_) => return Err("调整插入片段位置时 metadata 必须是对象".to_string()),
        None => json!({}),
    };
    let map = metadata
        .as_object_mut()
        .ok_or_else(|| "调整插入片段位置时 metadata 必须是对象".to_string())?;
    map.insert(
        "shot_number".to_string(),
        json!(insert_after_shot_number.unwrap_or(0)),
    );
    let entry = map
        .entry("assembly_insert".to_string())
        .or_insert_with(|| json!({}));
    let insert = entry
        .as_object_mut()
        .ok_or_else(|| "调整插入片段位置时 assembly_insert metadata 必须是对象".to_string())?;
    insert.insert("sort_order".to_string(), json!(sort_order));
    insert.insert(
        "insert_after_video_id".to_string(),
        json!(insert_after_video_id),
    );
    insert.insert(
        "insert_after_shot_number".to_string(),
        json!(insert_after_shot_number),
    );
    Ok(metadata)
}

pub(super) fn next_assembly_insert_sort_order(
    clips: &[AssemblyClip],
    insert_after_video_id: Option<i64>,
) -> Result<(f64, Option<i32>), String> {
    let Some(after_video_id) = insert_after_video_id else {
        let first_order = clips.first().map(|clip| clip.sort_order);
        return Ok((
            first_order.unwrap_or(ASSEMBLY_ORDER_STEP) - ASSEMBLY_ORDER_STEP / 2.0,
            None,
        ));
    };

    let index = clips
        .iter()
        .position(|clip| clip.video_id == after_video_id)
        .ok_or_else(|| format!("插入位置视频 #{after_video_id} 不在当前成片片段中"))?;
    let anchor = &clips[index];
    let next_order = clips.get(index + 1).map(|clip| clip.sort_order);
    let order = match next_order {
        Some(next) if next > anchor.sort_order => (anchor.sort_order + next) / 2.0,
        _ => anchor.sort_order + ASSEMBLY_ORDER_STEP / 2.0,
    };
    if !order.is_finite() {
        return Err("无法计算插入片段的成片顺序".to_string());
    }
    let anchor_shot_number = (anchor.shot_number > 0).then_some(anchor.shot_number);
    Ok((order, anchor_shot_number))
}

fn assembled_video_output_path(
    state: &AppState,
    project_id: i64,
    chapter_id: i64,
    workflow_mode: &str,
) -> Result<PathBuf, String> {
    let dir = ensure_chapter_videos_dir(state, project_id, chapter_id)?;
    Ok(dir.join(format!("assembled_{workflow_mode}.mp4")))
}

fn assembly_cache_manifest_path(output_path: &Path) -> PathBuf {
    let file_name = output_path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("assembled.mp4");
    output_path.with_file_name(format!("{file_name}.cache.json"))
}

fn build_assembly_cache_manifest(
    project_id: i64,
    chapter_id: i64,
    workflow_mode: &str,
    width: i32,
    height: i32,
    fps: i32,
    clips: &[AssemblyClip],
) -> Result<AssemblyCacheManifest, String> {
    build_assembly_cache_manifest_with_audio_probe(
        project_id,
        chapter_id,
        workflow_mode,
        width,
        height,
        fps,
        clips,
        &video_file_has_audio,
    )
}

pub(super) fn build_assembly_cache_manifest_with_audio_probe<F>(
    project_id: i64,
    chapter_id: i64,
    workflow_mode: &str,
    width: i32,
    height: i32,
    fps: i32,
    clips: &[AssemblyClip],
    has_audio: &F,
) -> Result<AssemblyCacheManifest, String>
where
    F: Fn(&Path) -> Result<bool, String>,
{
    let clips = clips
        .iter()
        .map(|clip| assembly_cache_clip(clip, has_audio))
        .collect::<Result<Vec<_>, _>>()?;
    let audio_enabled = clips.iter().any(|clip| clip.has_audio);
    Ok(AssemblyCacheManifest {
        version: 2,
        project_id,
        chapter_id,
        workflow_mode: workflow_mode.to_string(),
        width,
        height,
        fps,
        audio_enabled,
        clips,
    })
}

fn assembly_cache_clip<F>(clip: &AssemblyClip, has_audio: &F) -> Result<AssemblyCacheClip, String>
where
    F: Fn(&Path) -> Result<bool, String>,
{
    let metadata = std::fs::metadata(&clip.path)
        .map_err(|e| format!("无法读取成片片段视频 #{} 文件元数据：{e}", clip.video_id))?;
    if !metadata.is_file() {
        return Err(format!("成片片段视频 #{} 不是文件", clip.video_id));
    }
    let modified_unix_millis = file_modified_unix_millis(&metadata)
        .map_err(|e| format!("无法读取成片片段视频 #{} 修改时间：{e}", clip.video_id))?;
    Ok(AssemblyCacheClip {
        video_id: clip.video_id,
        video_type: clip.video_type.clone(),
        shot_number: clip.shot_number,
        sort_order: clip.sort_order,
        insert_after_video_id: clip.insert_after_video_id,
        file_path: clip.file_path.clone(),
        file_size: metadata.len(),
        modified_unix_millis,
        has_audio: has_audio(&clip.path)?,
        duration_secs: clip.duration_secs,
        width: clip.width,
        height: clip.height,
        fps: clip.fps,
    })
}

fn video_file_has_audio(path: &Path) -> Result<bool, String> {
    let output = Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-select_streams",
            "a:0",
            "-show_entries",
            "stream=index",
            "-of",
            "csv=p=0",
        ])
        .arg(path)
        .output()
        .map_err(|_| "未检测到 FFprobe，无法判断成片片段是否包含音频".to_string())?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "判断成片片段音频失败：{}",
            stderr.chars().take(500).collect::<String>()
        ));
    }
    Ok(!String::from_utf8_lossy(&output.stdout).trim().is_empty())
}

fn file_modified_unix_millis(metadata: &std::fs::Metadata) -> Result<u64, String> {
    let modified = metadata.modified().map_err(|e| e.to_string())?;
    let duration = modified
        .duration_since(UNIX_EPOCH)
        .map_err(|e| format!("文件修改时间早于 UNIX_EPOCH：{e}"))?;
    u64::try_from(duration.as_millis()).map_err(|_| "文件修改时间超过可记录范围".to_string())
}

pub(super) fn assembly_cache_matches(
    output_path: &Path,
    manifest_path: &Path,
    expected: &AssemblyCacheManifest,
) -> Result<bool, String> {
    if !output_path.is_file() || !manifest_path.is_file() {
        return Ok(false);
    }
    let bytes =
        std::fs::read(manifest_path).map_err(|e| format!("无法读取成片预览缓存清单：{e}"))?;
    let actual: AssemblyCacheManifest =
        serde_json::from_slice(&bytes).map_err(|e| format!("成片预览缓存清单格式无效：{e}"))?;
    Ok(&actual == expected)
}

pub(super) fn write_assembly_cache_manifest(
    manifest_path: &Path,
    manifest: &AssemblyCacheManifest,
) -> Result<(), String> {
    let bytes = serde_json::to_vec_pretty(manifest)
        .map_err(|e| format!("无法序列化成片预览缓存清单：{e}"))?;
    let tmp_path = manifest_path.with_extension("tmp");
    std::fs::write(&tmp_path, bytes).map_err(|e| format!("无法写入成片预览缓存清单：{e}"))?;
    std::fs::rename(&tmp_path, manifest_path).map_err(|e| format!("无法保存成片预览缓存清单：{e}"))
}

pub(super) fn assembly_dimensions(clips: &[AssemblyClip]) -> (i32, i32) {
    let first = clips.first();
    (
        even_positive_dimension(first.map(|clip| clip.width), 1280),
        even_positive_dimension(first.map(|clip| clip.height), 720),
    )
}

pub(super) fn assembly_fps(clips: &[AssemblyClip]) -> i32 {
    clips
        .first()
        .map(|clip| clip.fps)
        .filter(|fps| *fps > 0)
        .unwrap_or(24)
}

fn even_positive_dimension(value: Option<i32>, default_value: i32) -> i32 {
    let value = value.filter(|value| *value > 0).unwrap_or(default_value);
    if value % 2 == 0 { value } else { value + 1 }
}

pub(super) fn assembly_filter_complex(
    clips: &[AssemblyClip],
    width: i32,
    height: i32,
    fps: i32,
    manifest: &AssemblyCacheManifest,
) -> Result<(String, bool), String> {
    if clips.len() != manifest.clips.len() {
        return Err("成片缓存清单与片段数量不一致，无法拼接成片".to_string());
    }
    let clip_count = clips.len();
    let mut filter = String::new();
    for index in 0..clip_count {
        filter.push_str(&format!(
            "[{index}:v:0]scale={width}:{height}:force_original_aspect_ratio=decrease,pad={width}:{height}:(ow-iw)/2:(oh-ih)/2,setsar=1,fps={fps},format=yuv420p[v{index}];"
        ));
    }

    if manifest.audio_enabled {
        for (index, clip) in clips.iter().enumerate() {
            if manifest.clips[index].has_audio {
                filter.push_str(&format!(
                    "[{index}:a:0]aresample=async=1:first_pts=0,aformat=sample_fmts=fltp:sample_rates=48000:channel_layouts=stereo,asetpts=N/SR/TB[a{index}];"
                ));
            } else {
                filter.push_str(&format!(
                    "anullsrc=channel_layout=stereo:sample_rate=48000,atrim=duration={},asetpts=N/SR/TB[a{index}];",
                    ffmpeg_duration_literal(clip.duration_secs)
                ));
            }
        }
    }

    for index in 0..clip_count {
        if manifest.audio_enabled {
            filter.push_str(&format!("[v{index}][a{index}]"));
        } else {
            filter.push_str(&format!("[v{index}]"));
        }
    }
    if manifest.audio_enabled {
        filter.push_str(&format!("concat=n={clip_count}:v=1:a=1[outv][outa]"));
    } else {
        filter.push_str(&format!("concat=n={clip_count}:v=1:a=0[outv]"));
    }
    Ok((filter, manifest.audio_enabled))
}

fn ffmpeg_duration_literal(value: f32) -> String {
    let value = if value.is_finite() && value > 0.0 {
        value
    } else {
        0.1
    };
    format!("{value:.3}")
}

pub(super) fn run_ffmpeg_assemble_videos(
    clips: &[AssemblyClip],
    output_path: &Path,
    width: i32,
    height: i32,
    fps: i32,
    manifest: &AssemblyCacheManifest,
) -> Result<(), String> {
    ensure_ffmpeg().map_err(|err| format!("{err}，无法拼接成片"))?;
    let (filter, has_audio) = assembly_filter_complex(clips, width, height, fps, manifest)?;
    let mut command = Command::new("ffmpeg");
    command
        .arg("-y")
        .arg("-hide_banner")
        .arg("-loglevel")
        .arg("error");
    for clip in clips {
        command.arg("-i").arg(&clip.path);
    }
    command
        .arg("-filter_complex")
        .arg(filter)
        .arg("-map")
        .arg("[outv]")
        .arg("-c:v")
        .arg("libx264")
        .arg("-preset")
        .arg("veryfast")
        .arg("-crf")
        .arg("20")
        .arg("-pix_fmt")
        .arg("yuv420p");
    if has_audio {
        command
            .arg("-map")
            .arg("[outa]")
            .arg("-c:a")
            .arg("aac")
            .arg("-b:a")
            .arg("192k");
    } else {
        command.arg("-an");
    }
    let output = command
        .arg("-movflags")
        .arg("+faststart")
        .arg(output_path)
        .output()
        .map_err(|err| format!("拼接成片失败：未能启动 FFmpeg：{err}"))?;
    if output.status.success() {
        return Ok(());
    }
    let cleanup_error = remove_partial_video_file(output_path, "成片预览文件").err();
    let stderr = String::from_utf8_lossy(&output.stderr);
    Err(append_secondary_error(
        format!(
            "拼接成片失败：{}",
            stderr.chars().take(800).collect::<String>()
        ),
        cleanup_error,
    ))
}

async fn latest_postprocess_version(
    state: &AppState,
    push: &generated_video::Model,
) -> Result<generated_video::Model, String> {
    let chapter_id = push
        .chapter_id
        .ok_or_else(|| format!("推送批次 #{} 缺少章节绑定", push.id))?;
    let mut versions = Vec::new();
    for row in state
        .video
        .list_by_chapter(push.project_id, chapter_id)
        .await
        .map_err(|e| e.to_string())?
    {
        if row.video_type != ASSEMBLY_POSTPROCESS_EDIT_VIDEO_TYPE || row.status != "completed" {
            continue;
        }
        let PostprocessEditMetadata {
            push_video_id,
            base_video_id,
        } = parse_postprocess_edit_metadata(&row)?;
        if push_video_id != push.id {
            continue;
        }
        if row.source_video_id != Some(base_video_id) {
            return Err(format!(
                "后处理编辑视频 #{} 的 source_video_id 与 postprocess_edit.base_video_id 不一致",
                row.id
            ));
        }
        if row.workflow_mode.as_deref() != push.workflow_mode.as_deref() {
            return Err(format!(
                "后处理编辑视频 #{} 的 workflow_mode 与推送批次 #{} 不一致",
                row.id, push.id
            ));
        }
        versions.push(row);
    }
    versions.sort_by(|left, right| {
        left.created_at
            .cmp(&right.created_at)
            .then_with(|| left.id.cmp(&right.id))
    });
    Ok(versions.pop().unwrap_or_else(|| push.clone()))
}

pub(super) fn parse_postprocess_edit_metadata(
    video: &generated_video::Model,
) -> Result<PostprocessEditMetadata, String> {
    let metadata = video
        .metadata_json
        .as_ref()
        .and_then(|metadata| metadata.get("postprocess_edit"))
        .and_then(Value::as_object)
        .ok_or_else(|| {
            format!(
                "后处理编辑视频 #{} 缺少 postprocess_edit metadata",
                video.id
            )
        })?;
    let push_video_id = required_positive_i64(metadata.get("push_video_id")).map_err(|err| {
        format!(
            "后处理编辑视频 #{} 的 postprocess_edit.push_video_id {err}",
            video.id
        )
    })?;
    let base_video_id = required_positive_i64(metadata.get("base_video_id")).map_err(|err| {
        format!(
            "后处理编辑视频 #{} 的 postprocess_edit.base_video_id {err}",
            video.id
        )
    })?;
    let _operation = metadata
        .get("operation")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            format!(
                "后处理编辑视频 #{} 缺少 postprocess_edit.operation",
                video.id
            )
        })
        .and_then(|value| validate_postprocess_operation(value).map(str::to_string))?;
    let _source_video_id =
        optional_positive_i64(metadata.get("source_video_id")).map_err(|err| {
            format!(
                "后处理编辑视频 #{} 的 postprocess_edit.source_video_id {err}",
                video.id
            )
        })?;
    Ok(PostprocessEditMetadata {
        push_video_id,
        base_video_id,
    })
}

fn required_positive_i64(value: Option<&Value>) -> Result<i64, String> {
    value
        .and_then(Value::as_i64)
        .filter(|id| *id > 0)
        .ok_or_else(|| "必须是正整数".to_string())
}

pub(super) fn validate_postprocess_operation(value: &str) -> Result<&'static str, String> {
    match value.trim() {
        "insert" => Ok("insert"),
        "delete" => Ok("delete"),
        "replace" => Ok("replace"),
        "" => Err("请选择后处理操作".to_string()),
        other => Err(format!("后处理操作无效：{other}")),
    }
}

async fn postprocess_edit_source(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    input: &ApplyAssemblyPostprocessEditInput,
    operation: &str,
) -> Result<Option<PostprocessEditSource>, String> {
    if operation == "delete" {
        return Ok(None);
    }
    let has_uploaded_source = [
        input.source_file_name.as_deref(),
        input.source_mime_type.as_deref(),
        input.source_data_base64.as_deref(),
    ]
    .iter()
    .any(|value| value.map(str::trim).is_some_and(|value| !value.is_empty()));
    let source_count = [input.source_video_id.is_some(), has_uploaded_source]
        .into_iter()
        .filter(|selected| *selected)
        .count();
    if source_count > 1 {
        return Err("插入或替换后处理片段只能选择一个来源视频".to_string());
    }
    if let Some(video_id) = input.source_video_id {
        let video = state
            .video
            .get_video(video_id)
            .await
            .map_err(|e| e.to_string())?;
        access::verify_project_editor_string(state, user_id, video.project_id).await?;
        if video.project_id != project_id {
            return Err(format!("替换/插入来源视频 #{} 不属于当前项目", video.id));
        }
        if video.status != "completed" {
            return Err(format!("替换/插入来源视频 #{} 尚未完成", video.id));
        }
        let file_path = video
            .file_path
            .as_deref()
            .map(str::trim)
            .filter(|path| !path.is_empty())
            .ok_or_else(|| format!("替换/插入来源视频 #{} 没有本地文件", video.id))?;
        let path = canonicalize_data_file_path(
            &state.config.data_dir,
            file_path,
            &format!("替换/插入来源视频 #{}", video.id),
        )?;
        return Ok(Some(PostprocessEditSource {
            path,
            file_name: video
                .file_path
                .as_deref()
                .and_then(|path| Path::new(path).file_name())
                .and_then(|value| value.to_str())
                .map(ToString::to_string),
            source_video_id: Some(video.id),
            _uploaded_source: None,
        }));
    }
    if has_uploaded_source {
        let file_name = input
            .source_file_name
            .as_deref()
            .ok_or_else(|| "上传后处理来源视频缺少文件名".to_string())?;
        let mime_type = input
            .source_mime_type
            .as_deref()
            .ok_or_else(|| "上传后处理来源视频缺少 MIME".to_string())?;
        let data_base64 = input
            .source_data_base64
            .as_deref()
            .ok_or_else(|| "上传后处理来源视频缺少数据".to_string())?;
        let source =
            uploaded_video_source(state, file_name, mime_type, data_base64, "后处理来源视频")?;
        return Ok(Some(PostprocessEditSource {
            path: source.path.clone(),
            file_name: Some(source.file_name.clone()),
            source_video_id: None,
            _uploaded_source: Some(source),
        }));
    }
    Err("插入或替换后处理片段时必须选择上传视频或 API 生成视频".to_string())
}

pub(super) fn postprocess_edit_range(
    base: &generated_video::Model,
    input: &ApplyAssemblyPostprocessEditInput,
    operation: &str,
    source: Option<&PostprocessEditSource>,
) -> Result<(f32, f32, Option<f32>), String> {
    let base_duration = base.duration_secs.max(0.1);
    let source_duration = if let Some(source) = source {
        Some(probe_video_properties(&source.path, "后处理来源视频")?.duration_secs)
    } else {
        None
    };
    match operation {
        "insert" => {
            let at = input
                .at_secs
                .ok_or_else(|| "插入操作缺少插入时间".to_string())?;
            if !at.is_finite() || at < 0.0 || at > base_duration {
                return Err("插入时间超出当前片段范围".to_string());
            }
            Ok((at, at, source_duration))
        }
        "delete" | "replace" => {
            let start = input
                .start_secs
                .ok_or_else(|| "删除/替换操作缺少起始时间".to_string())?;
            let end = input
                .end_secs
                .ok_or_else(|| "删除/替换操作缺少结束时间".to_string())?;
            if !start.is_finite()
                || !end.is_finite()
                || start < 0.0
                || end <= start
                || end > base_duration + 0.05
            {
                return Err("删除/替换时间范围无效".to_string());
            }
            Ok((start, end.min(base_duration), source_duration))
        }
        _ => Err(format!("后处理操作无效：{operation}")),
    }
}

struct FfmpegPostprocessEditInput<'a> {
    base_path: &'a Path,
    source_path: Option<&'a Path>,
    output_path: &'a Path,
    operation: &'a str,
    start_secs: f32,
    end_secs: f32,
    base_duration_secs: f32,
    width: i32,
    height: i32,
    fps: i32,
}

fn run_ffmpeg_postprocess_edit(input: FfmpegPostprocessEditInput<'_>) -> Result<(), String> {
    let FfmpegPostprocessEditInput {
        base_path,
        source_path,
        output_path,
        operation,
        start_secs,
        end_secs,
        base_duration_secs,
        width,
        height,
        fps,
    } = input;
    ensure_ffmpeg().map_err(|err| format!("{err}，无法执行后处理剪辑"))?;
    let width = even_positive_dimension(Some(width), 1280);
    let height = even_positive_dimension(Some(height), 720);
    let fps = if fps > 0 { fps } else { 24 };
    let base_duration_secs = base_duration_secs.max(0.1);
    let start_secs = start_secs.clamp(0.0, base_duration_secs);
    let end_secs = end_secs.clamp(start_secs, base_duration_secs);
    let mut filter = String::new();
    let mut labels = Vec::new();
    if start_secs > 0.01 {
        filter.push_str(&format!(
            "[0:v:0]trim=start=0:end={},setpts=PTS-STARTPTS,scale={width}:{height}:force_original_aspect_ratio=decrease,pad={width}:{height}:(ow-iw)/2:(oh-ih)/2,setsar=1,fps={fps},format=yuv420p[v0];",
            ffmpeg_duration_literal(start_secs)
        ));
        labels.push("[v0]".to_string());
    }
    if matches!(operation, "insert" | "replace") {
        let Some(source_path) = source_path else {
            return Err("插入或替换操作缺少来源视频".to_string());
        };
        if !source_path.is_file() {
            return Err("插入或替换来源视频不是文件".to_string());
        }
        filter.push_str(&format!(
            "[1:v:0]scale={width}:{height}:force_original_aspect_ratio=decrease,pad={width}:{height}:(ow-iw)/2:(oh-ih)/2,setsar=1,fps={fps},format=yuv420p[vsrc];"
        ));
        labels.push("[vsrc]".to_string());
    }
    let tail_start = if operation == "insert" {
        start_secs
    } else {
        end_secs
    };
    if tail_start < base_duration_secs - 0.01 {
        filter.push_str(&format!(
            "[0:v:0]trim=start={}:end={},setpts=PTS-STARTPTS,scale={width}:{height}:force_original_aspect_ratio=decrease,pad={width}:{height}:(ow-iw)/2:(oh-ih)/2,setsar=1,fps={fps},format=yuv420p[vtail];",
            ffmpeg_duration_literal(tail_start),
            ffmpeg_duration_literal(base_duration_secs)
        ));
        labels.push("[vtail]".to_string());
    }
    if labels.is_empty() {
        return Err("后处理剪辑结果为空".to_string());
    }
    for label in &labels {
        filter.push_str(label);
    }
    filter.push_str(&format!("concat=n={}:v=1:a=0[outv]", labels.len()));

    let mut command = Command::new("ffmpeg");
    command
        .arg("-y")
        .arg("-hide_banner")
        .arg("-loglevel")
        .arg("error")
        .arg("-i")
        .arg(base_path);
    if matches!(operation, "insert" | "replace") {
        command
            .arg("-i")
            .arg(source_path.expect("source path checked above"));
    }
    let output = command
        .arg("-filter_complex")
        .arg(filter)
        .arg("-map")
        .arg("[outv]")
        .arg("-c:v")
        .arg("libx264")
        .arg("-preset")
        .arg("veryfast")
        .arg("-crf")
        .arg("20")
        .arg("-pix_fmt")
        .arg("yuv420p")
        .arg("-an")
        .arg("-movflags")
        .arg("+faststart")
        .arg(output_path)
        .output()
        .map_err(|err| format!("后处理剪辑失败：未能启动 FFmpeg：{err}"))?;
    if output.status.success() {
        return Ok(());
    }
    let cleanup_error = remove_partial_video_file(output_path, "后处理剪辑输出文件").err();
    let stderr = String::from_utf8_lossy(&output.stderr);
    Err(append_secondary_error(
        format!(
            "后处理剪辑失败：{}",
            stderr.chars().take(800).collect::<String>()
        ),
        cleanup_error,
    ))
}
