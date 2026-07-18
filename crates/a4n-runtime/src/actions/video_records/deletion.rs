use super::assembly::{is_assembly_clip_video_type, pushed_source_ids_for_chapter};
use super::contracts::DeleteChapterVideoAssetsResult;
use super::imports::{delete_video_record_then_files, video_delete_file_paths};
use crate::AppState;
use crate::actions::access;
use crate::actions::app_error_message;
use crate::actions::video_generation::media::delete_video_tail_frame_assets;
use crate::actions::video_prompts::resolve_project_chapter_video_source;

pub async fn delete_video(state: &AppState, user_id: i64, video_id: i64) -> Result<(), String> {
    let existing = match state
        .orchestrator
        .generated_video_by_id(video_id)
        .await
        .map_err(app_error_message)?
    {
        Some(existing) => existing,
        None => {
            let project_ids = state
                .orchestrator
                .missing_video_reference_project_ids(video_id)
                .await
                .map_err(app_error_message)?;
            if project_ids.is_empty() {
                return Err("视频不存在".to_string());
            }
            for project_id in project_ids {
                access::verify_project_editor_string(state, user_id, project_id).await?;
            }
            state
                .orchestrator
                .delete_missing_generated_video_references(video_id)
                .await
                .map_err(app_error_message)?;
            return Ok(());
        }
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
            .orchestrator
            .delete_generated_video_record(video_id)
            .await
            .map_err(app_error_message)
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
    let video_ids = state
        .orchestrator
        .generated_video_ids_for_chapter(project_id, chapter_id)
        .await
        .map_err(app_error_message)?;
    let mut deleted_video_count = 0;
    for video_id in video_ids {
        delete_video(state, user_id, video_id).await?;
        deleted_video_count += 1;
    }
    Ok(DeleteChapterVideoAssetsResult {
        deleted_video_count,
    })
}
