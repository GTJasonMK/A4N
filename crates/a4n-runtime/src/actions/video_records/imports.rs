use super::assembly::{
    ASSEMBLY_INSERT_VIDEO_TYPE, AssemblyCacheClip, AssemblyCacheManifest, AssemblyClip,
    assembly_clips, assembly_push_rows, ensure_insert_position_not_inside_pushed_prefix,
    next_assembly_insert_sort_order, run_ffmpeg_assemble_videos, validate_assembly_workflow_mode,
};
use crate::AppState;
use crate::actions::access;
use crate::actions::image_generation::{GeneratedImageSummary, to_summary};
use crate::actions::video_generation::media::{
    append_secondary_error, best_effort_video_thumbnail_path, capture_video_tail_frame_asset,
    combine_optional_errors, delete_video_tail_frame_assets, ensure_chapter_videos_dir,
    ensure_ffmpeg, remove_partial_video_file, run_ffmpeg_trim, trimmed_video_path,
};
use crate::actions::video_generation::video_to_summary;
use crate::actions::video_generation::{
    GeneratedVideoSummary, reference_media_dimensions_for_aspect_ratio,
};
use crate::actions::video_prompts::{
    ensure_prompt_matches_current_chapter, resolve_project_chapter_video_source,
};
use crate::file_utils::{
    canonicalize_data_file_path, data_dir_relative_file_path, deletable_data_file_path,
};
use a4n_core::files::paths::ProjectPaths;
use a4n_core::models::ai::generated_video;
use a4n_core::models::chapter::chapter_video_prompt;
use a4n_services::image::CreateImageRecord;
use a4n_services::video::CreateVideoRecord;
use base64::Engine as _;
use chrono::Utc;
use sea_orm::EntityTrait;
use serde::Deserialize;
use serde_json::{Value, json};
use std::future::Future;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::process::Command;

const MAX_UPLOAD_VIDEO_BYTES: usize = 512 * 1024 * 1024;

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UploadChapterVideoInput {
    pub file_name: String,
    pub mime_type: String,
    pub data_base64: String,
    pub prompt_id: i64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UploadReferenceMediaVideoInput {
    pub file_name: String,
    pub mime_type: String,
    pub data_base64: String,
    pub prompt: String,
    pub duration_secs: i32,
    pub aspect_ratio: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UploadAssemblyInsertVideoInput {
    pub file_name: String,
    pub mime_type: String,
    pub data_base64: String,
    #[serde(default)]
    pub insert_after_video_id: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImportCanvasVideoInput {
    #[serde(default)]
    pub file_path: Option<String>,
    #[serde(default)]
    pub file_name: Option<String>,
    #[serde(default)]
    pub mime_type: Option<String>,
    #[serde(default)]
    pub data_base64: Option<String>,
    #[serde(default)]
    pub prompt: Option<String>,
}

pub(super) struct ImportedVideoSource {
    pub(super) path: PathBuf,
    file_size: i64,
    pub(super) file_name: String,
    extension: &'static str,
    temporary_path: Option<PathBuf>,
}

impl Drop for ImportedVideoSource {
    fn drop(&mut self) {
        if let Some(path) = self.temporary_path.as_deref() {
            let _ = std::fs::remove_file(path);
        }
    }
}
pub async fn upload_chapter_video(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    chapter_id: i64,
    input: UploadChapterVideoInput,
) -> Result<GeneratedVideoSummary, String> {
    let source = uploaded_video_source(
        state,
        &input.file_name,
        &input.mime_type,
        &input.data_base64,
        "导入视频",
    )?;
    create_imported_chapter_video(
        state,
        user_id,
        project_id,
        chapter_id,
        input.prompt_id,
        source,
    )
    .await
}

pub async fn import_canvas_video(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    input: ImportCanvasVideoInput,
) -> Result<GeneratedVideoSummary, String> {
    access::verify_project_editor_string(state, user_id, project_id).await?;
    let project = state
        .project
        .get(project_id)
        .await
        .map_err(|e| e.to_string())?;
    if project.project_kind != "video" {
        return Err("画布视频导入只支持视频项目".to_string());
    }
    let source = match input
        .file_path
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
    {
        Some(file_path) => import_video_source(file_path, "画布视频导入")?,
        None => {
            let file_name = input
                .file_name
                .as_deref()
                .ok_or_else(|| "上传画布视频缺少 file_name".to_string())?;
            let mime_type = input
                .mime_type
                .as_deref()
                .ok_or_else(|| "上传画布视频缺少 mime_type".to_string())?;
            let data_base64 = input
                .data_base64
                .as_deref()
                .ok_or_else(|| "上传画布视频缺少 data_base64".to_string())?;
            uploaded_video_source(state, file_name, mime_type, data_base64, "画布视频导入")?
        }
    };
    create_imported_canvas_video(state, project_id, input.prompt, source).await
}

pub async fn extract_canvas_video_frame(
    state: &AppState,
    user_id: i64,
    video_id: i64,
    frame_kind: String,
    at_secs: Option<f32>,
) -> Result<GeneratedImageSummary, String> {
    let video = state
        .video
        .get_video(video_id)
        .await
        .map_err(|e| e.to_string())?;
    access::verify_project_editor_string(state, user_id, video.project_id).await?;
    if video.chapter_id.is_some() {
        return Err("ExtractFrameNode 只支持项目级画布视频".to_string());
    }
    let source_path = video
        .file_path
        .as_deref()
        .ok_or_else(|| "源视频还没有可抽帧的本地文件".to_string())?;
    let source_path = canonicalize_data_file_path(
        &state.config.data_dir,
        source_path,
        &format!("源视频 #{}", video.id),
    )?;
    ensure_ffmpeg()?;

    let frame_kind = normalize_canvas_frame_kind(&frame_kind)?;
    let timestamp = canvas_frame_timestamp(&video, &frame_kind, at_secs)?;
    let output = canvas_frame_output_path(state, video.project_id, video.id, &frame_kind)?;
    if let Err(err) = run_ffmpeg_extract_frame(&source_path, &output, timestamp) {
        return Err(append_secondary_error(
            err,
            remove_partial_video_file(&output, "画布抽帧图片").err(),
        ));
    }
    let output_path_str =
        data_dir_relative_file_path(&state.config.data_dir, &output, "画布抽帧图片")?;
    let file_size = std::fs::metadata(&output)
        .map_err(|err| format!("无法读取画布抽帧图片元数据：{err}"))?
        .len() as i64;
    let metadata = json!({
        "canvas_frame": {
            "source_video_id": video.id,
            "frame_kind": frame_kind,
            "at_secs": timestamp,
            "source_workflow_mode": video.workflow_mode,
            "extracted_at": Utc::now().to_rfc3339(),
        },
        "source": "video_canvas",
    });
    let image = match state
        .image
        .create_record(CreateImageRecord {
            project_id: video.project_id,
            image_type: "canvas_frame".to_string(),
            file_path: output_path_str,
            prompt: format!("视频 #{} 画布抽帧 {frame_kind}", video.id),
            model: "ffmpeg".to_string(),
            width: video.width.max(1),
            height: video.height.max(1),
            file_size: Some(file_size),
            chapter_id: None,
            panel_id: None,
            metadata_json: Some(metadata),
            created_at: None,
        })
        .await
    {
        Ok(image) => image,
        Err(err) => {
            return Err(append_secondary_error(
                format!("画布抽帧图片记录写入失败：{err}"),
                remove_partial_video_file(&output, "画布抽帧图片").err(),
            ));
        }
    };
    Ok(to_summary(&image))
}

pub async fn assemble_canvas_videos(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    video_ids: Vec<i64>,
    title: Option<String>,
) -> Result<GeneratedVideoSummary, String> {
    access::verify_project_editor_string(state, user_id, project_id).await?;
    if video_ids.is_empty() {
        return Err("AssembleNode 至少需要一个视频片段".to_string());
    }
    let mut seen = std::collections::HashSet::new();
    let mut clips = Vec::with_capacity(video_ids.len());
    for (index, video_id) in video_ids.iter().copied().enumerate() {
        if !seen.insert(video_id) {
            return Err(format!("AssembleNode 视频片段重复: {video_id}"));
        }
        let video = state
            .video
            .get_video(video_id)
            .await
            .map_err(|e| e.to_string())?;
        if video.project_id != project_id {
            return Err(format!("视频 #{video_id} 不属于项目 {project_id}"));
        }
        if video.chapter_id.is_some() {
            return Err(format!("AssembleNode 只支持项目级画布视频: {video_id}"));
        }
        if video.status != "completed" {
            return Err(format!("视频 #{video_id} 尚未完成，不能成片"));
        }
        let file_path = video
            .file_path
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| format!("视频 #{video_id} 缺少本地文件"))?;
        let path = canonicalize_data_file_path(
            &state.config.data_dir,
            file_path,
            &format!("画布成片片段视频 #{video_id}"),
        )?;
        clips.push(AssemblyClip {
            video_id,
            video_type: video.video_type.clone(),
            shot_number: i32::try_from(index + 1).unwrap_or(i32::MAX),
            sort_order: index as f64,
            insert_after_video_id: None,
            path,
            file_path: file_path.to_string(),
            duration_secs: video.duration_secs,
            width: video.width,
            height: video.height,
            fps: video.fps,
        });
    }
    let first = clips
        .first()
        .ok_or_else(|| "AssembleNode 至少需要一个视频片段".to_string())?;
    let width = first.width.max(1);
    let height = first.height.max(1);
    let fps = first.fps.max(1);
    let duration_secs = clips
        .iter()
        .map(|clip| clip.duration_secs.max(0.0))
        .sum::<f32>();
    let title = title
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("画布成片")
        .to_string();
    let metadata = json!({
        "canvas_assembly": {
            "source_video_ids": video_ids,
            "clip_count": clips.len(),
            "assembled_at": Utc::now().to_rfc3339(),
        },
        "source": "video_canvas",
    });
    let record = state
        .video
        .create_video_record(CreateVideoRecord {
            project_id,
            chapter_id: None,
            video_type: "canvas_assembly".to_string(),
            workflow_mode: Some("canvas_assembly".to_string()),
            prompt: title,
            negative_prompt: None,
            model: "ffmpeg".to_string(),
            width,
            height,
            duration_secs,
            fps,
            source_video_id: None,
            source_image_id: None,
            provider_job_id: None,
            metadata_json: Some(metadata),
        })
        .await
        .map_err(|e| e.to_string())?;
    let output =
        import_project_video_output_path(state, project_id, record.id, "canvas_assembly", "mp4")?;
    let manifest = canvas_assembly_manifest(project_id, &clips, width, height, fps);
    if let Err(err) = run_ffmpeg_assemble_videos(&clips, &output, width, height, fps, &manifest) {
        let mark_failed_error = state
            .video
            .mark_failed_and_update_prompt(record.id, None, &err)
            .await
            .map_err(|e| e.to_string())
            .err();
        return Err(append_secondary_error(err, mark_failed_error));
    }
    let output_path_str = data_dir_relative_file_path(&state.config.data_dir, &output, "画布成片")?;
    let file_size = std::fs::metadata(&output)
        .map_err(|err| format!("无法读取画布成片文件元数据：{err}"))?
        .len() as i64;
    let updated = mark_imported_video_completed(
        state,
        record.id,
        &output,
        &output_path_str,
        None,
        file_size,
        None,
        None,
        None,
        "更新画布成片记录失败",
        "画布成片文件",
        "画布成片缩略图",
    )
    .await?;
    Ok(video_to_summary(&updated))
}

async fn create_imported_canvas_video(
    state: &AppState,
    project_id: i64,
    prompt: Option<String>,
    source: ImportedVideoSource,
) -> Result<GeneratedVideoSummary, String> {
    let probed = probe_video_properties(&source.path, "画布导入视频").ok();
    let prompt = prompt
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("导入画布视频")
        .to_string();
    let metadata = json!({
        "canvas_import": {
            "external": true,
            "source_file_name": &source.file_name,
            "source_extension": source.extension,
            "imported_at": Utc::now().to_rfc3339()
        }
    });
    let record = state
        .video
        .create_video_record(CreateVideoRecord {
            project_id,
            chapter_id: None,
            video_type: "canvas_video".to_string(),
            workflow_mode: Some("canvas_video".to_string()),
            prompt,
            negative_prompt: None,
            model: "external-import".to_string(),
            width: probed.as_ref().map(|value| value.width).unwrap_or(1280),
            height: probed.as_ref().map(|value| value.height).unwrap_or(720),
            duration_secs: probed
                .as_ref()
                .map(|value| value.duration_secs)
                .unwrap_or(0.0),
            fps: probed.as_ref().map(|value| value.fps).unwrap_or(24),
            source_video_id: None,
            source_image_id: None,
            provider_job_id: None,
            metadata_json: Some(metadata),
        })
        .await
        .map_err(|e| e.to_string())?;

    let output = import_project_video_output_path(
        state,
        project_id,
        record.id,
        "imported_canvas_video",
        source.extension,
    )?;
    copy_imported_video_or_delete_record(
        state,
        record.id,
        &source.path,
        &output,
        "复制画布导入视频失败",
    )
    .await?;
    let output_path_str =
        data_dir_relative_file_path(&state.config.data_dir, &output, "画布导入视频")?;
    let updated = mark_imported_video_completed(
        state,
        record.id,
        &output,
        &output_path_str,
        None,
        source.file_size,
        None,
        None,
        None,
        "更新画布导入视频记录失败",
        "画布导入视频文件",
        "画布导入视频缩略图",
    )
    .await?;
    Ok(video_to_summary(&updated))
}

async fn create_imported_chapter_video(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    chapter_id: i64,
    prompt_id: i64,
    source: ImportedVideoSource,
) -> Result<GeneratedVideoSummary, String> {
    access::verify_project_editor_string(state, user_id, project_id).await?;
    resolve_project_chapter_video_source(state, project_id, chapter_id, "导入视频", true).await?;
    let prompt = import_video_prompt(state, project_id, chapter_id, prompt_id).await?;
    let properties = probe_video_properties(&source.path, "导入视频")?;
    let prompt_text = imported_video_prompt_text(&prompt);
    let metadata = imported_chapter_video_metadata(
        &prompt,
        &source.file_name,
        source.extension,
        Utc::now().to_rfc3339(),
    );

    let record = state
        .video
        .create_video_record(CreateVideoRecord {
            project_id,
            chapter_id: Some(chapter_id),
            video_type: "chapter_video".into(),
            workflow_mode: Some(prompt.workflow_mode.clone()),
            prompt: prompt_text,
            negative_prompt: prompt.negative_prompt.clone(),
            model: "external-import".to_string(),
            width: properties.width,
            height: properties.height,
            duration_secs: properties.duration_secs,
            fps: properties.fps,
            source_video_id: None,
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
        "imported_video",
        source.extension,
    )?;
    copy_imported_video_or_delete_record(
        state,
        record.id,
        &source.path,
        &output,
        "复制导入视频失败",
    )
    .await?;
    let output_path_str = data_dir_relative_file_path(&state.config.data_dir, &output, "导入视频")?;
    let thumbnail_path = best_effort_video_thumbnail_path(
        &state.config.data_dir,
        project_id,
        chapter_id,
        record.id,
        &output,
    );
    let mut tail_frame_capture_error = None;
    let tail_frame_metadata =
        if should_capture_imported_chapter_video_tail_frame(&prompt.workflow_mode) {
            match capture_video_tail_frame_asset(
                state,
                &record,
                chapter_id,
                Some(prompt.id),
                &output,
            )
            .await
            {
                Ok(metadata) => Some(metadata),
                Err(err) => {
                    if prompt.workflow_mode == "first_frame_chain" {
                        let cleanup_error = combine_optional_errors(
                            remove_partial_video_file(&output, "导入视频文件").err(),
                            thumbnail_path.as_deref().and_then(|path| {
                                remove_partial_video_file(
                                    &state.config.data_dir.join(path),
                                    "视频缩略图",
                                )
                                .err()
                            }),
                        );
                        let cleanup_error = combine_optional_errors(
                            cleanup_error,
                            delete_video_tail_frame_assets(state, record.id).await.err(),
                        );
                        let delete_error = delete_video_record_error(state, record.id).await;
                        return Err(append_secondary_error(
                            append_secondary_error(
                                format!("导入视频已复制但首帧串联尾帧采集失败：{err}"),
                                cleanup_error,
                            ),
                            delete_error,
                        ));
                    }
                    tail_frame_capture_error = Some(err);
                    None
                }
            }
        } else {
            None
        };
    let updated = mark_imported_video_completed(
        state,
        record.id,
        &output,
        &output_path_str,
        thumbnail_path.as_deref(),
        source.file_size,
        Some(prompt.id),
        tail_frame_metadata.as_ref(),
        tail_frame_capture_error.as_deref(),
        "导入视频记录完成失败",
        "导入视频文件",
        "视频缩略图",
    )
    .await?;
    Ok(video_to_summary(&updated))
}

fn should_capture_imported_chapter_video_tail_frame(workflow_mode: &str) -> bool {
    matches!(
        workflow_mode,
        "boundary_frames" | "first_frame_chain" | "chat_video"
    )
}

pub async fn upload_reference_media_video(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    chapter_id: i64,
    input: UploadReferenceMediaVideoInput,
) -> Result<GeneratedVideoSummary, String> {
    let source = uploaded_video_source(
        state,
        &input.file_name,
        &input.mime_type,
        &input.data_base64,
        "导入参考媒体视频",
    )?;
    create_imported_reference_media_video(
        state,
        user_id,
        project_id,
        chapter_id,
        input.prompt,
        input.duration_secs,
        input.aspect_ratio,
        source,
    )
    .await
}

async fn create_imported_reference_media_video(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    chapter_id: i64,
    prompt: String,
    duration_secs: i32,
    aspect_ratio: String,
    source: ImportedVideoSource,
) -> Result<GeneratedVideoSummary, String> {
    access::verify_project_editor_string(state, user_id, project_id).await?;
    resolve_project_chapter_video_source(state, project_id, chapter_id, "导入参考媒体视频", true)
        .await?;
    let prompt = prompt.trim();
    if prompt.is_empty() {
        return Err("参考媒体视频 prompt 不能为空".to_string());
    }
    let import_options = validate_reference_media_import_options(duration_secs, &aspect_ratio)?;
    let properties = probe_video_properties(&source.path, "导入参考媒体视频")?;
    let metadata = json!({
        "provider_backend": "external-import",
        "reference_media": {
            "functionMode": "omni_reference",
            "aspect_ratio": &import_options.aspect_ratio,
        },
        "import": {
            "external": true,
            "source_file_name": &source.file_name,
            "source_extension": source.extension,
            "imported_at": Utc::now().to_rfc3339(),
        }
    });

    let record = state
        .video
        .create_video_record(CreateVideoRecord {
            project_id,
            chapter_id: Some(chapter_id),
            video_type: "reference_media_video".into(),
            workflow_mode: Some("reference_media".to_string()),
            prompt: prompt.to_string(),
            negative_prompt: None,
            model: "external-reference-media-import".to_string(),
            width: properties.width,
            height: properties.height,
            duration_secs: properties.duration_secs,
            fps: properties.fps,
            source_video_id: None,
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
        "reference_media_import",
        source.extension,
    )?;
    copy_imported_video_or_delete_record(
        state,
        record.id,
        &source.path,
        &output,
        "复制导入参考媒体视频失败",
    )
    .await?;
    let output_path_str =
        data_dir_relative_file_path(&state.config.data_dir, &output, "导入参考媒体视频")?;
    let thumbnail_path = best_effort_video_thumbnail_path(
        &state.config.data_dir,
        project_id,
        chapter_id,
        record.id,
        &output,
    );
    let updated = mark_imported_video_completed(
        state,
        record.id,
        &output,
        &output_path_str,
        thumbnail_path.as_deref(),
        source.file_size,
        None,
        None,
        None,
        "导入参考媒体视频记录完成失败",
        "导入参考媒体视频文件",
        "视频缩略图",
    )
    .await?;
    Ok(video_to_summary(&updated))
}

pub async fn upload_assembly_insert_video(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    chapter_id: i64,
    workflow_mode: String,
    input: UploadAssemblyInsertVideoInput,
) -> Result<GeneratedVideoSummary, String> {
    let source = uploaded_video_source(
        state,
        &input.file_name,
        &input.mime_type,
        &input.data_base64,
        "插入视频",
    )?;
    create_assembly_insert_video(
        state,
        user_id,
        project_id,
        chapter_id,
        workflow_mode,
        input.insert_after_video_id,
        source,
    )
    .await
}

async fn create_assembly_insert_video(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    chapter_id: i64,
    workflow_mode: String,
    insert_after_video_id: Option<i64>,
    source: ImportedVideoSource,
) -> Result<GeneratedVideoSummary, String> {
    access::verify_project_editor_string(state, user_id, project_id).await?;
    resolve_project_chapter_video_source(state, project_id, chapter_id, "导入衔接片段", true)
        .await?;
    let workflow_mode = validate_assembly_workflow_mode(&workflow_mode)?;
    let properties = probe_video_properties(&source.path, "插入视频")?;
    let rows = state
        .video
        .list_by_chapter(project_id, chapter_id)
        .await
        .map_err(|e| e.to_string())?;
    let clips = assembly_clips(state, rows.clone(), project_id, chapter_id, workflow_mode)?;
    let pushes = assembly_push_rows(&rows, workflow_mode)?;
    ensure_insert_position_not_inside_pushed_prefix(&clips, &pushes, insert_after_video_id)?;
    let (sort_order, insert_after_shot_number) =
        next_assembly_insert_sort_order(&clips, insert_after_video_id)?;
    let imported_at = Utc::now().to_rfc3339();
    let anchor_label = insert_after_video_id
        .map(|id| format!("视频 #{id} 之后"))
        .unwrap_or_else(|| "成片开头".to_string());
    let metadata = json!({
        "shot_number": insert_after_shot_number.unwrap_or(0),
        "assembly_insert": {
            "sort_order": sort_order,
            "insert_after_video_id": insert_after_video_id,
            "insert_after_shot_number": insert_after_shot_number,
            "source_file_name": &source.file_name,
            "source_extension": source.extension,
            "imported_at": imported_at,
        }
    });

    let record = state
        .video
        .create_video_record(CreateVideoRecord {
            project_id,
            chapter_id: Some(chapter_id),
            video_type: ASSEMBLY_INSERT_VIDEO_TYPE.to_string(),
            workflow_mode: Some(workflow_mode.to_string()),
            prompt: format!("成片衔接片段，插入位置：{anchor_label}。"),
            negative_prompt: None,
            model: "external-assembly-insert".to_string(),
            width: properties.width,
            height: properties.height,
            duration_secs: properties.duration_secs,
            fps: properties.fps,
            source_video_id: None,
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
        "assembly_insert",
        source.extension,
    )?;
    copy_imported_video_or_delete_record(
        state,
        record.id,
        &source.path,
        &output,
        "复制插入视频失败",
    )
    .await?;
    let output_path_str = data_dir_relative_file_path(&state.config.data_dir, &output, "插入视频")?;
    let thumbnail_path = best_effort_video_thumbnail_path(
        &state.config.data_dir,
        project_id,
        chapter_id,
        record.id,
        &output,
    );
    let updated = mark_imported_video_completed(
        state,
        record.id,
        &output,
        &output_path_str,
        thumbnail_path.as_deref(),
        source.file_size,
        None,
        None,
        None,
        "插入视频记录完成失败",
        "插入视频文件",
        "插入视频缩略图",
    )
    .await?;
    Ok(video_to_summary(&updated))
}
pub(super) async fn delete_video_record_then_files<F, Fut>(
    file_paths: Vec<(&'static str, PathBuf)>,
    delete_record: F,
) -> Result<(), String>
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = Result<(), String>>,
{
    delete_record().await?;
    remove_video_delete_files(&file_paths)
}

async fn import_video_prompt(
    state: &AppState,
    project_id: i64,
    chapter_id: i64,
    prompt_id: i64,
) -> Result<chapter_video_prompt::Model, String> {
    let prompt = chapter_video_prompt::Entity::find_by_id(prompt_id)
        .one(state.db.as_ref())
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("视频提示词不存在: {prompt_id}"))?;
    if prompt.project_id != project_id {
        return Err(format!(
            "视频提示词 #{} 不属于当前项目 {}",
            prompt.id, project_id
        ));
    }
    if prompt.chapter_id != chapter_id {
        return Err(format!(
            "视频提示词 #{} 不属于当前章节 {}",
            prompt.id, chapter_id
        ));
    }
    if prompt.prompt_type != "video_prompt" {
        return Err("请选择 video_prompt 类型的提示词导入视频".to_string());
    }
    if prompt.status == "running" {
        return Err(format!("视频提示词 #{} 正在运行，不能导入视频", prompt.id));
    }
    if prompt.generated_video_id.is_some() {
        return Err("该视频提示词已关联视频，请先删除对应视频后再导入".to_string());
    }
    ensure_prompt_matches_current_chapter(state, &prompt).await?;
    Ok(prompt)
}

fn import_video_source(file_path: &str, label: &str) -> Result<ImportedVideoSource, String> {
    let path = import_video_source_path(file_path)?;
    let metadata = std::fs::metadata(&path).map_err(|e| format!("无法读取{label}元数据：{e}"))?;
    let file_size =
        i64::try_from(metadata.len()).map_err(|_| format!("{label}文件过大，无法记录文件大小"))?;
    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("video")
        .to_string();
    let extension = import_video_extension(&path)?;
    Ok(ImportedVideoSource {
        path,
        file_size,
        file_name,
        extension,
        temporary_path: None,
    })
}

pub(super) fn uploaded_video_source(
    state: &AppState,
    file_name: &str,
    mime_type: &str,
    data_base64: &str,
    label: &str,
) -> Result<ImportedVideoSource, String> {
    let file_name = required_upload_video_file_name(file_name)?;
    let extension = import_video_extension(Path::new(&file_name))?;
    validate_upload_video_mime(mime_type, extension)?;
    let raw = strip_data_url_prefix(data_base64).trim();
    if raw.is_empty() {
        return Err(format!("{label}数据不能为空"));
    }
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(raw)
        .map_err(|err| format!("{label} base64 解码失败：{err}"))?;
    if bytes.is_empty() {
        return Err(format!("{label}文件不能为空"));
    }
    if bytes.len() > MAX_UPLOAD_VIDEO_BYTES {
        return Err(format!(
            "{label}不能超过 {}MB",
            MAX_UPLOAD_VIDEO_BYTES / 1024 / 1024
        ));
    }

    let upload_dir = state.config.temp_dir.join("video_uploads");
    std::fs::create_dir_all(&upload_dir).map_err(|e| format!("创建视频上传临时目录失败：{e}"))?;
    let stamp = Utc::now()
        .timestamp_nanos_opt()
        .unwrap_or_else(|| Utc::now().timestamp_micros() * 1000);
    let temp_path = upload_dir.join(format!(
        "video_upload_{}_{}.{}",
        std::process::id(),
        stamp,
        extension
    ));
    std::fs::write(&temp_path, bytes).map_err(|e| format!("写入{label}临时文件失败：{e}"))?;

    let mut source = import_video_source(
        temp_path
            .to_str()
            .ok_or_else(|| "视频上传临时路径不是有效 UTF-8".to_string())?,
        label,
    )
    .inspect_err(|_err| {
        let _ = std::fs::remove_file(&temp_path);
    })?;
    source.file_name = file_name;
    source.temporary_path = Some(temp_path);
    Ok(source)
}

async fn copy_imported_video_or_delete_record(
    state: &AppState,
    record_id: i64,
    source: &Path,
    output: &Path,
    failure_prefix: &str,
) -> Result<(), String> {
    if let Err(err) = std::fs::copy(source, output) {
        let delete_error = delete_video_record_error(state, record_id).await;
        return Err(append_secondary_error(
            format!("{failure_prefix}：{err}"),
            delete_error,
        ));
    }
    Ok(())
}

pub(super) async fn delete_video_record_error(state: &AppState, record_id: i64) -> Option<String> {
    state
        .video
        .delete(record_id)
        .await
        .err()
        .map(|e| e.to_string())
}

pub(super) async fn mark_imported_video_completed(
    state: &AppState,
    record_id: i64,
    output: &Path,
    output_path_str: &str,
    thumbnail_path: Option<&str>,
    file_size: i64,
    prompt_id: Option<i64>,
    tail_frame_metadata: Option<&Value>,
    tail_frame_capture_error: Option<&str>,
    failure_prefix: &str,
    file_label: &str,
    thumbnail_label: &str,
) -> Result<generated_video::Model, String> {
    let completion_metadata = match (tail_frame_metadata, tail_frame_capture_error) {
        (Some(metadata), None) => Some(json!({ "tail_frame": metadata })),
        (Some(metadata), Some(error)) => {
            Some(json!({ "tail_frame": metadata, "tail_frame_capture_error": error }))
        }
        (None, Some(error)) => Some(json!({ "tail_frame_capture_error": error })),
        (None, None) => None,
    };
    match state
        .video
        .mark_completed_and_link_prompt(
            record_id,
            output_path_str,
            thumbnail_path,
            file_size,
            completion_metadata,
            prompt_id,
        )
        .await
    {
        Ok(updated) => Ok(updated),
        Err(err) => {
            let cleanup_error = combine_optional_errors(
                remove_partial_video_file(output, file_label).err(),
                thumbnail_path.and_then(|path| {
                    remove_partial_video_file(&state.config.data_dir.join(path), thumbnail_label)
                        .err()
                }),
            );
            let cleanup_error = combine_optional_errors(
                cleanup_error,
                delete_video_tail_frame_assets(state, record_id).await.err(),
            );
            let delete_error = delete_video_record_error(state, record_id).await;
            Err(append_secondary_error(
                append_secondary_error(format!("{failure_prefix}：{err}"), cleanup_error),
                delete_error,
            ))
        }
    }
}

pub(super) fn import_video_source_path(file_path: &str) -> Result<PathBuf, String> {
    let trimmed = file_path.trim();
    if trimmed.is_empty() {
        return Err("请选择要导入的视频文件".to_string());
    }
    let path = PathBuf::from(trimmed);
    let metadata = std::fs::metadata(&path).map_err(|e| format!("无法读取导入视频文件：{e}"))?;
    if !metadata.is_file() {
        return Err("导入路径不是文件".to_string());
    }
    import_video_extension(&path)?;
    path.canonicalize()
        .map_err(|e| format!("无法解析导入视频路径：{e}"))
}

pub(super) fn import_video_extension(path: &Path) -> Result<&'static str, String> {
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("导入视频缺少文件扩展名：{}", path.display()))?;
    match extension.to_ascii_lowercase().as_str() {
        "mp4" => Ok("mp4"),
        "webm" => Ok("webm"),
        "mov" => Ok("mov"),
        "m4v" => Ok("m4v"),
        _ => Err(format!("导入视频格式不支持：{extension}")),
    }
}

fn required_upload_video_file_name(file_name: &str) -> Result<String, String> {
    let trimmed = file_name.trim();
    if trimmed.is_empty() {
        return Err("上传视频文件名不能为空".to_string());
    }
    let name = Path::new(trimmed)
        .file_name()
        .and_then(|value| value.to_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "上传视频文件名无效".to_string())?;
    if name != trimmed {
        return Err("上传视频文件名不能包含路径".to_string());
    }
    Ok(name.to_string())
}

fn validate_upload_video_mime(mime_type: &str, extension: &str) -> Result<(), String> {
    let mime = mime_type.trim().to_ascii_lowercase();
    let valid = match extension {
        "mp4" => matches!(mime.as_str(), "video/mp4" | "application/mp4"),
        "webm" => mime == "video/webm",
        "mov" => matches!(mime.as_str(), "video/quicktime" | "video/mov"),
        "m4v" => matches!(
            mime.as_str(),
            "video/x-m4v" | "video/mp4" | "application/mp4"
        ),
        _ => false,
    };
    if valid {
        Ok(())
    } else {
        Err("上传视频 MIME 与文件扩展名不匹配".to_string())
    }
}

fn strip_data_url_prefix(value: &str) -> &str {
    value
        .split_once(";base64,")
        .map(|(_, body)| body)
        .unwrap_or(value)
}

pub(super) fn import_video_output_path(
    state: &AppState,
    project_id: i64,
    chapter_id: i64,
    video_id: i64,
    prefix: &str,
    extension: &str,
) -> Result<PathBuf, String> {
    let dir = ensure_chapter_videos_dir(state, project_id, chapter_id)?;
    Ok(dir.join(format!("{prefix}_{video_id}.{extension}")))
}

fn import_project_video_output_path(
    state: &AppState,
    project_id: i64,
    video_id: i64,
    prefix: &str,
    extension: &str,
) -> Result<PathBuf, String> {
    let dir = ProjectPaths::new(&state.config.data_dir, project_id).videos_dir();
    std::fs::create_dir_all(&dir).map_err(|err| format!("无法创建项目视频目录：{err}"))?;
    Ok(dir.join(format!("{prefix}_{video_id}.{extension}")))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ReferenceMediaImportOptions {
    pub(super) aspect_ratio: String,
}

pub(super) fn validate_reference_media_import_options(
    duration_secs: i32,
    aspect_ratio: &str,
) -> Result<ReferenceMediaImportOptions, String> {
    if !(1..=60).contains(&duration_secs) {
        return Err("参考媒体视频时长必须在 1 到 60 秒之间".to_string());
    }
    let aspect_ratio = aspect_ratio.trim();
    reference_media_dimensions_for_aspect_ratio(aspect_ratio)?;
    Ok(ReferenceMediaImportOptions {
        aspect_ratio: aspect_ratio.to_string(),
    })
}

#[derive(Debug, Clone, Copy)]
pub(super) struct VideoProbeProperties {
    pub(super) width: i32,
    pub(super) height: i32,
    pub(super) duration_secs: f32,
    pub(super) fps: i32,
}

pub(super) fn probe_video_properties(
    path: &Path,
    label: &str,
) -> Result<VideoProbeProperties, String> {
    let output = Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-select_streams",
            "v:0",
            "-show_entries",
            "stream=width,height,r_frame_rate",
            "-show_entries",
            "format=duration",
            "-of",
            "json",
        ])
        .arg(path)
        .output()
        .map_err(|_| format!("未检测到 FFprobe，无法读取{label}元数据"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "读取{label}元数据失败：{}",
            stderr.chars().take(500).collect::<String>()
        ));
    }
    parse_video_probe_properties(&output.stdout, label)
}

pub(super) fn parse_video_probe_properties(
    stdout: &[u8],
    label: &str,
) -> Result<VideoProbeProperties, String> {
    let value: Value = serde_json::from_slice(stdout)
        .map_err(|err| format!("{label}元数据不是有效 JSON：{err}"))?;
    let stream = value
        .get("streams")
        .and_then(Value::as_array)
        .and_then(|streams| streams.first())
        .ok_or_else(|| format!("{label}缺少视频流"))?;
    let width = stream
        .get("width")
        .and_then(Value::as_i64)
        .and_then(|value| i32::try_from(value).ok())
        .filter(|value| *value > 0)
        .ok_or_else(|| format!("{label}缺少有效宽度"))?;
    let height = stream
        .get("height")
        .and_then(Value::as_i64)
        .and_then(|value| i32::try_from(value).ok())
        .filter(|value| *value > 0)
        .ok_or_else(|| format!("{label}缺少有效高度"))?;
    let duration_secs = value
        .get("format")
        .and_then(|format| format.get("duration"))
        .and_then(Value::as_str)
        .and_then(|value| value.parse::<f32>().ok())
        .filter(|value| value.is_finite() && *value > 0.0)
        .ok_or_else(|| format!("{label}缺少有效时长"))?;
    let fps = stream
        .get("r_frame_rate")
        .and_then(Value::as_str)
        .and_then(parse_frame_rate)
        .unwrap_or(24);

    Ok(VideoProbeProperties {
        width,
        height,
        duration_secs,
        fps,
    })
}

pub(super) fn parse_frame_rate(value: &str) -> Option<i32> {
    let value = value.trim();
    if value.is_empty() {
        return None;
    }
    let fps = if let Some((left, right)) = value.split_once('/') {
        let numerator = left.trim().parse::<f64>().ok()?;
        let denominator = right.trim().parse::<f64>().ok()?;
        if denominator <= 0.0 {
            return None;
        }
        numerator / denominator
    } else {
        value.parse::<f64>().ok()?
    };
    let rounded = fps.round();
    if !rounded.is_finite() || rounded <= 0.0 || rounded > f64::from(i32::MAX) {
        return None;
    }
    Some(rounded as i32)
}

fn imported_video_prompt_text(prompt: &chapter_video_prompt::Model) -> String {
    let mut sections = vec![prompt.visual_prompt.trim().to_string()];
    if let Some(motion) = prompt
        .motion_prompt
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
    {
        sections.push(format!("Motion Prompt:\n{motion}"));
    }
    sections.join("\n\n")
}

fn remove_video_delete_files(file_paths: &[(&'static str, PathBuf)]) -> Result<(), String> {
    for (label, path) in file_paths {
        match std::fs::remove_file(path) {
            Ok(()) => {}
            Err(err) if err.kind() == ErrorKind::NotFound => {}
            Err(err) => return Err(format!("{label}删除失败：{}：{err}", path.display())),
        }
    }
    Ok(())
}

pub(super) fn video_delete_file_paths(
    data_dir: &Path,
    file_path: Option<&str>,
    thumbnail_path: Option<&str>,
) -> Result<Vec<(&'static str, PathBuf)>, String> {
    let mut out = Vec::new();
    for (label, path) in [("视频文件", file_path), ("视频缩略图", thumbnail_path)] {
        let Some(path) = path.map(str::trim).filter(|path| !path.is_empty()) else {
            continue;
        };
        let Some(canonical) = deletable_data_file_path(data_dir, path, label)? else {
            continue;
        };
        if !out
            .iter()
            .any(|(_, existing_path)| existing_path == &canonical)
        {
            out.push((label, canonical));
        }
    }
    Ok(out)
}

pub async fn trim_video(
    state: &AppState,
    user_id: i64,
    video_id: i64,
    start_secs: f32,
    end_secs: f32,
) -> Result<GeneratedVideoSummary, String> {
    validate_video_trim_range(start_secs, end_secs)?;
    let source = state
        .video
        .get_video(video_id)
        .await
        .map_err(|e| e.to_string())?;
    access::verify_project_editor_string(state, user_id, source.project_id).await?;
    let source_workflow_mode = source.workflow_mode.clone().ok_or_else(|| {
        format!(
            "源视频 #{} 缺少 workflow_mode，无法按生成模式保存裁剪结果",
            source.id
        )
    })?;
    let target_workflow_mode =
        if source.chapter_id.is_none() || source_workflow_mode.starts_with("canvas_") {
            "canvas_trim".to_string()
        } else {
            source_workflow_mode
        };
    let source_path = source
        .file_path
        .clone()
        .ok_or_else(|| "源视频还没有可裁剪的本地文件".to_string())?;
    let source_path = canonicalize_data_file_path(
        &state.config.data_dir,
        &source_path,
        &format!("源视频 #{}", source.id),
    )?;
    if source.duration_secs > 0.0 && end_secs > source.duration_secs + 0.5 {
        return Err("裁剪结束时间超过视频时长".into());
    }
    ensure_ffmpeg()?;

    let output = trimmed_video_path(state, &source, start_secs, end_secs)?;
    if let Err(err) = run_ffmpeg_trim(&source_path, &output, start_secs, end_secs) {
        return Err(append_secondary_error(
            err,
            remove_partial_video_file(&output, "裁剪输出文件").err(),
        ));
    }
    let size = std::fs::metadata(&output)
        .map(|m| m.len() as i64)
        .map_err(|e| format!("无法读取裁剪视频文件元数据：{e}"))?;

    let record = match state
        .video
        .create_video_record(CreateVideoRecord {
            project_id: source.project_id,
            chapter_id: source.chapter_id,
            video_type: "trimmed_clip".into(),
            workflow_mode: Some(target_workflow_mode),
            prompt: source.prompt.clone(),
            negative_prompt: source.negative_prompt.clone(),
            model: source.model.clone(),
            width: source.width,
            height: source.height,
            duration_secs: end_secs - start_secs,
            fps: source.fps,
            source_video_id: Some(source.id),
            source_image_id: source.source_image_id,
            provider_job_id: None,
            metadata_json: Some(trimmed_video_metadata(&source, start_secs, end_secs)),
        })
        .await
    {
        Ok(record) => record,
        Err(err) => {
            return Err(append_secondary_error(
                err.to_string(),
                remove_partial_video_file(&output, "裁剪输出文件").err(),
            ));
        }
    };
    let output_path_str = data_dir_relative_file_path(&state.config.data_dir, &output, "裁剪视频")?;
    let thumbnail_path = source.chapter_id.and_then(|chapter_id| {
        best_effort_video_thumbnail_path(
            &state.config.data_dir,
            source.project_id,
            chapter_id,
            record.id,
            &output,
        )
    });
    let updated = match state
        .video
        .mark_completed_and_link_prompt(
            record.id,
            &output_path_str,
            thumbnail_path.as_deref(),
            size,
            None,
            None,
        )
        .await
    {
        Ok(updated) => updated,
        Err(err) => {
            let cleanup_error = combine_optional_errors(
                remove_partial_video_file(&output, "裁剪输出文件").err(),
                thumbnail_path.as_deref().and_then(|path| {
                    remove_partial_video_file(&state.config.data_dir.join(path), "视频缩略图").err()
                }),
            );
            let mark_failed_error = state
                .video
                .mark_failed_and_update_prompt(
                    record.id,
                    None,
                    &format!("裁剪文件已生成但记录完成状态失败：{err}"),
                )
                .await
                .map_err(|e| e.to_string())
                .err();
            let msg = append_secondary_error(err.to_string(), cleanup_error);
            return Err(append_secondary_error(msg, mark_failed_error));
        }
    };
    Ok(video_to_summary(&updated))
}

pub(super) fn trimmed_video_metadata(
    source: &generated_video::Model,
    start_secs: f32,
    end_secs: f32,
) -> Value {
    let mut metadata = json!({
        "trim": {
            "start_secs": start_secs,
            "end_secs": end_secs,
        }
    });
    insert_source_context(&mut metadata, source.metadata_json.as_ref());
    metadata
}

pub(super) fn imported_chapter_video_metadata(
    prompt: &chapter_video_prompt::Model,
    source_file_name: &str,
    source_extension: &str,
    imported_at: String,
) -> Value {
    let mut metadata = json!({
        "prompt_id": prompt.id,
        "shot_number": prompt.shot_number,
        "source_prompt_type": prompt.prompt_type,
        "import": {
            "external": true,
            "source_file_name": source_file_name,
            "source_extension": source_extension,
            "imported_at": imported_at,
        }
    });
    insert_source_context(&mut metadata, prompt.metadata_json.as_ref());
    metadata
}

fn insert_source_context(metadata: &mut Value, source_metadata: Option<&Value>) {
    if let Some(source_context) = source_metadata
        .and_then(|metadata| metadata.get("source"))
        .cloned()
        && let Some(object) = metadata.as_object_mut()
    {
        object.insert("source".to_string(), source_context);
    }
}

fn normalize_canvas_frame_kind(value: &str) -> Result<String, String> {
    let value = value.trim();
    match value {
        "" | "first" | "first_frame" => Ok("first".to_string()),
        "tail" | "last" | "last_frame" | "end" => Ok("tail".to_string()),
        "time" | "key" | "key_frame" | "at_secs" => Ok("time".to_string()),
        other => Err(format!("抽帧类型无效: {other}")),
    }
}

fn canvas_frame_timestamp(
    video: &generated_video::Model,
    frame_kind: &str,
    at_secs: Option<f32>,
) -> Result<f32, String> {
    let duration = video.duration_secs.max(0.0);
    let timestamp = match frame_kind {
        "first" => 0.0,
        "tail" => {
            if duration > 0.12 {
                duration - 0.08
            } else {
                0.0
            }
        }
        "time" => at_secs.ok_or_else(|| "按时间抽帧必须提供 at_secs".to_string())?,
        _ => return Err(format!("抽帧类型无效: {frame_kind}")),
    };
    if !timestamp.is_finite() || timestamp < 0.0 {
        return Err("抽帧时间必须是非负数".to_string());
    }
    if duration > 0.0 && timestamp > duration + 0.5 {
        return Err("抽帧时间超过视频时长".to_string());
    }
    Ok(timestamp)
}

fn canvas_frame_output_path(
    state: &AppState,
    project_id: i64,
    video_id: i64,
    frame_kind: &str,
) -> Result<PathBuf, String> {
    let dir = ProjectPaths::new(&state.config.data_dir, project_id).images_dir();
    std::fs::create_dir_all(&dir).map_err(|err| format!("无法创建画布抽帧图片目录：{err}"))?;
    let stamp = Utc::now().timestamp_millis();
    Ok(dir.join(format!(
        "canvas_frame_video_{video_id}_{frame_kind}_{stamp}.jpg"
    )))
}

fn run_ffmpeg_extract_frame(video_path: &Path, output: &Path, at_secs: f32) -> Result<(), String> {
    let result = Command::new("ffmpeg")
        .arg("-y")
        .arg("-ss")
        .arg(format!("{:.3}", at_secs.max(0.0)))
        .arg("-i")
        .arg(video_path)
        .arg("-frames:v")
        .arg("1")
        .arg("-q:v")
        .arg("2")
        .arg("-update")
        .arg("1")
        .arg(output)
        .output()
        .map_err(|err| format!("画布抽帧失败：未能启动 FFmpeg：{err}"))?;
    if !result.status.success() {
        let stderr = String::from_utf8_lossy(&result.stderr);
        return Err(format!(
            "画布抽帧失败：{}",
            stderr.chars().take(500).collect::<String>()
        ));
    }
    Ok(())
}

fn canvas_assembly_manifest(
    project_id: i64,
    clips: &[AssemblyClip],
    width: i32,
    height: i32,
    fps: i32,
) -> AssemblyCacheManifest {
    AssemblyCacheManifest {
        version: 1,
        project_id,
        chapter_id: 0,
        workflow_mode: "canvas_assembly".to_string(),
        width,
        height,
        fps,
        audio_enabled: false,
        clips: clips
            .iter()
            .map(|clip| {
                let metadata = std::fs::metadata(&clip.path).ok();
                let modified_unix_millis = metadata
                    .as_ref()
                    .and_then(|metadata| metadata.modified().ok())
                    .and_then(|modified| modified.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|duration| duration.as_millis() as u64)
                    .unwrap_or(0);
                AssemblyCacheClip {
                    video_id: clip.video_id,
                    video_type: clip.video_type.clone(),
                    shot_number: clip.shot_number,
                    sort_order: clip.sort_order,
                    insert_after_video_id: clip.insert_after_video_id,
                    file_path: clip.file_path.clone(),
                    file_size: metadata.as_ref().map(std::fs::Metadata::len).unwrap_or(0),
                    modified_unix_millis,
                    has_audio: false,
                    duration_secs: clip.duration_secs,
                    width: clip.width,
                    height: clip.height,
                    fps: clip.fps,
                }
            })
            .collect(),
    }
}

pub(super) fn validate_video_trim_range(start_secs: f32, end_secs: f32) -> Result<(), String> {
    if !start_secs.is_finite()
        || !end_secs.is_finite()
        || start_secs < 0.0
        || end_secs <= start_secs
    {
        return Err("裁剪时间范围无效".into());
    }
    Ok(())
}
