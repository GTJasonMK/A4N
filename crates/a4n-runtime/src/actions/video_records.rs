use crate::AppState;
use crate::actions::access;
use crate::actions::video_generation::media::{
    delete_video_tail_frame_assets, video_mime_from_path,
};
use crate::actions::video_generation::{GeneratedVideoSummary, video_to_summary};
use crate::actions::video_prompts::resolve_project_chapter_video_source;
use crate::file_utils::canonicalize_data_file_path;
use a4n_core::error::AppError;
use a4n_core::models::ai::generated_video;
use a4n_services::video::UpdateVideoRecord;
use base64::Engine as _;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder};
use serde::{Deserialize, Serialize};
use serde_json::Value;

mod assembly;
mod imports;

pub use self::assembly::{
    ApplyAssemblyPostprocessEditInput, AssembledChapterVideoClipSummary,
    AssembledChapterVideoPreview, AssemblyPushResult, PushAssembledChapterVideoInput,
    UpdateAssemblyPushStatusInput, apply_assembly_postprocess_edit, assemble_chapter_videos,
    load_cached_assembled_chapter_video_preview, push_assembled_chapter_video,
    update_assembly_push_status,
};
use self::assembly::{
    assembly_clips, assembly_push_rows, ensure_insert_position_not_inside_pushed_prefix,
    is_assembly_clip_video_type, pushed_source_ids_for_chapter,
    resolve_assembly_insert_position_metadata,
};
pub use self::imports::{
    ImportCanvasVideoInput, UploadAssemblyInsertVideoInput, UploadChapterVideoInput,
    UploadReferenceMediaVideoInput, assemble_canvas_videos, extract_canvas_video_frame,
    import_canvas_video, trim_video, upload_assembly_insert_video, upload_chapter_video,
    upload_reference_media_video,
};
use self::imports::{delete_video_record_then_files, video_delete_file_paths};

const MAX_INLINE_PLAYBACK_VIDEO_BYTES: u64 = 256 * 1024 * 1024;
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoPlaybackData {
    pub mime_type: String,
    pub data_base64: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct DeleteChapterVideoAssetsResult {
    pub deleted_video_count: u64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpdateGeneratedVideoInput {
    #[serde(default)]
    pub prompt: Option<String>,
    #[serde(default)]
    pub negative_prompt: Option<String>,
    #[serde(default)]
    pub clear_negative_prompt: bool,
    #[serde(default)]
    pub metadata_json: Option<Value>,
    #[serde(default)]
    pub clear_metadata_json: bool,
    #[serde(default)]
    pub is_favorite: Option<bool>,
    #[serde(default)]
    pub insert_after_video_id: Option<i64>,
    #[serde(default)]
    pub move_assembly_insert_to_start: bool,
}

pub async fn list_videos_by_chapter(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    chapter_id: i64,
) -> Result<Vec<GeneratedVideoSummary>, String> {
    access::verify_project_access_string(state, user_id, project_id).await?;
    resolve_project_chapter_video_source(state, project_id, chapter_id, "查看章节视频", false)
        .await?;
    let rows = state
        .video
        .list_by_chapter(project_id, chapter_id)
        .await
        .map_err(|e| e.to_string())?;
    Ok(rows.iter().map(video_to_summary).collect())
}

pub async fn toggle_video_favorite(
    state: &AppState,
    user_id: i64,
    video_id: i64,
) -> Result<GeneratedVideoSummary, String> {
    let video = state
        .video
        .get_video(video_id)
        .await
        .map_err(|e| e.to_string())?;
    access::verify_project_editor_string(state, user_id, video.project_id).await?;
    let row = state
        .video
        .toggle_favorite(video_id)
        .await
        .map_err(|e| e.to_string())?;
    Ok(video_to_summary(&row))
}

pub async fn update_video(
    state: &AppState,
    user_id: i64,
    video_id: i64,
    input: UpdateGeneratedVideoInput,
) -> Result<GeneratedVideoSummary, String> {
    let existing = state
        .video
        .get_video(video_id)
        .await
        .map_err(|e| e.to_string())?;
    access::verify_project_editor_string(state, user_id, existing.project_id).await?;
    let modifies_pushed_source = input.prompt.is_some()
        || input.negative_prompt.is_some()
        || input.clear_negative_prompt
        || input.metadata_json.is_some()
        || input.clear_metadata_json
        || input.insert_after_video_id.is_some()
        || input.move_assembly_insert_to_start;
    if modifies_pushed_source
        && is_assembly_clip_video_type(&existing.video_type)
        && let (Some(chapter_id), Some(workflow_mode)) = (
            existing.chapter_id,
            existing.workflow_mode.as_deref().map(str::trim),
        )
        && pushed_source_ids_for_chapter(state, existing.project_id, chapter_id, workflow_mode)
            .await?
            .contains(&existing.id)
    {
        return Err("该视频片段已推送到成片后处理，不能修改".to_string());
    }

    let mut metadata_json = input.metadata_json;
    let mut clear_metadata_json = input.clear_metadata_json;
    let should_move_insert =
        input.move_assembly_insert_to_start || input.insert_after_video_id.is_some();
    if should_move_insert {
        if clear_metadata_json {
            return Err("调整插入片段位置时不能同时清空视频 metadata".to_string());
        }
        let target_after_video_id = if input.move_assembly_insert_to_start {
            None
        } else {
            input.insert_after_video_id
        };
        if let Some(chapter_id) = existing.chapter_id {
            let workflow_mode = existing
                .workflow_mode
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .ok_or_else(|| format!("插入片段 #{} 缺少 workflow_mode", existing.id))?;
            let rows = state
                .video
                .list_by_chapter(existing.project_id, chapter_id)
                .await
                .map_err(|e| e.to_string())?;
            let clips = assembly_clips(
                state,
                rows.iter()
                    .filter(|row| row.id != existing.id)
                    .cloned()
                    .collect(),
                existing.project_id,
                chapter_id,
                workflow_mode,
            )?;
            let pushes = assembly_push_rows(&rows, workflow_mode)?;
            ensure_insert_position_not_inside_pushed_prefix(
                &clips,
                &pushes,
                target_after_video_id,
            )?;
        }
        metadata_json = Some(
            resolve_assembly_insert_position_metadata(
                state,
                &existing,
                metadata_json.or_else(|| existing.metadata_json.clone()),
                target_after_video_id,
            )
            .await?,
        );
        clear_metadata_json = false;
    }

    let row = state
        .video
        .update_record(
            video_id,
            UpdateVideoRecord {
                prompt: input.prompt,
                negative_prompt: input.negative_prompt,
                clear_negative_prompt: input.clear_negative_prompt,
                metadata_json,
                clear_metadata_json,
                is_favorite: input.is_favorite,
            },
        )
        .await
        .map_err(|e| e.to_string())?;
    Ok(video_to_summary(&row))
}

pub async fn load_video_playback_data(
    state: &AppState,
    user_id: i64,
    video_id: i64,
) -> Result<VideoPlaybackData, String> {
    let video = state
        .video
        .get_video(video_id)
        .await
        .map_err(|e| e.to_string())?;
    access::verify_project_editor_string(state, user_id, video.project_id).await?;
    let file_path = video
        .file_path
        .as_deref()
        .map(str::trim)
        .filter(|path| !path.is_empty())
        .ok_or_else(|| format!("视频 #{} 没有本地文件", video.id))?;
    let path = canonicalize_data_file_path(
        &state.config.data_dir,
        file_path,
        &format!("视频 #{}", video.id),
    )?;
    let metadata = std::fs::metadata(&path).map_err(|e| format!("无法读取视频文件元数据：{e}"))?;
    if metadata.len() > MAX_INLINE_PLAYBACK_VIDEO_BYTES {
        return Err(format!(
            "视频文件超过 {}MB，无法以内置播放器 Blob 方式加载",
            MAX_INLINE_PLAYBACK_VIDEO_BYTES / 1024 / 1024
        ));
    }
    let bytes = std::fs::read(&path).map_err(|e| format!("无法读取视频文件：{e}"))?;
    Ok(VideoPlaybackData {
        mime_type: video_mime_from_path(&path)?.to_string(),
        data_base64: base64::engine::general_purpose::STANDARD.encode(bytes),
    })
}

pub async fn delete_video(state: &AppState, user_id: i64, video_id: i64) -> Result<(), String> {
    let existing = match state.video.get_video(video_id).await {
        Ok(existing) => existing,
        Err(AppError::NotFound(_)) => {
            let project_ids = state
                .video
                .missing_video_reference_project_ids(video_id)
                .await
                .map_err(|e| e.to_string())?;
            if project_ids.is_empty() {
                return Err("视频不存在".to_string());
            }
            for project_id in project_ids {
                access::verify_project_editor_string(state, user_id, project_id).await?;
            }
            state
                .video
                .delete_missing_video_references(video_id)
                .await
                .map_err(|e| e.to_string())?;
            return Ok(());
        }
        Err(err) => return Err(err.to_string()),
    };
    access::verify_project_editor_string(state, user_id, existing.project_id).await?;
    if is_assembly_clip_video_type(&existing.video_type)
        && let (Some(chapter_id), Some(workflow_mode)) = (
            existing.chapter_id,
            existing.workflow_mode.as_deref().map(str::trim),
        )
        && pushed_source_ids_for_chapter(state, existing.project_id, chapter_id, workflow_mode)
            .await?
            .contains(&existing.id)
    {
        return Err("该视频片段已推送到成片后处理，不能删除".to_string());
    }
    let file_paths = video_delete_file_paths(
        &state.config.data_dir,
        existing.file_path.as_deref(),
        existing.thumbnail_path.as_deref(),
    )?;
    delete_video_record_then_files(file_paths, || async move {
        delete_video_tail_frame_assets(state, video_id).await?;
        state
            .video
            .delete(video_id)
            .await
            .map_err(|e| e.to_string())
    })
    .await
}

pub async fn delete_chapter_video_assets(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    chapter_id: i64,
) -> Result<DeleteChapterVideoAssetsResult, String> {
    access::verify_project_editor_string(state, user_id, project_id).await?;
    resolve_project_chapter_video_source(state, project_id, chapter_id, "批量删除视频资产", true)
        .await?;
    let videos = generated_video::Entity::find()
        .filter(generated_video::Column::ProjectId.eq(project_id))
        .filter(generated_video::Column::ChapterId.eq(chapter_id))
        .order_by_asc(generated_video::Column::Id)
        .all(state.db.as_ref())
        .await
        .map_err(|e| e.to_string())?;
    let mut deleted_video_count = 0;
    for video in videos {
        delete_video(state, user_id, video.id).await?;
        deleted_video_count += 1;
    }
    Ok(DeleteChapterVideoAssetsResult {
        deleted_video_count,
    })
}

#[cfg(test)]
mod tests;
