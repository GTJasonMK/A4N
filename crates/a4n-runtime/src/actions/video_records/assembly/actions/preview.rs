use super::super::cache::{
    assembly_cache_matches, run_ffmpeg_assemble_videos, write_assembly_cache_manifest,
};
use super::super::contracts::{
    AssembledChapterVideoClipSummary, AssembledChapterVideoPreview, AssemblyClip,
    AssemblyRequestContext,
};
use super::super::push::assembly_request_context;
use crate::AppState;
use crate::actions::video_generation::media::{
    append_secondary_error, combine_optional_errors, remove_partial_video_file,
    video_mime_from_path,
};
use crate::actions::video_records::MAX_INLINE_PLAYBACK_VIDEO_BYTES;
use crate::file_utils::data_dir_relative_file_path;
use base64::Engine;
use chrono::Utc;
use std::path::Path;

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

    persist_assembly_preview_cache(&context.output_path, &context.manifest_path, || {
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
    })
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

pub(in crate::actions::video_records) fn persist_assembly_preview_cache<T, F>(
    output_path: &Path,
    manifest_path: &Path,
    operation: F,
) -> Result<T, String>
where
    F: FnOnce() -> Result<T, String>,
{
    match operation() {
        Ok(value) => Ok(value),
        Err(error) => {
            let cleanup_error = combine_optional_errors(
                remove_partial_video_file(output_path, "成片预览文件").err(),
                remove_partial_video_file(manifest_path, "成片预览缓存清单").err(),
            );
            Err(append_secondary_error(error, cleanup_error))
        }
    }
}
