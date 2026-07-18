use super::assembly::{
    assembly_clips, assembly_push_rows, ensure_insert_position_not_inside_pushed_prefix,
    is_assembly_clip_video_type, pushed_source_ids_for_chapter,
    resolve_assembly_insert_position_metadata,
};
use super::contracts::UpdateGeneratedVideoInput;
use crate::AppState;
use crate::actions::access;
use crate::actions::app_error_message;
use crate::actions::video_generation::{GeneratedVideoSummary, video_to_summary};
use crate::actions::video_prompts::resolve_project_chapter_video_source;
use a4n_core::models::ai::generated_video;
use a4n_orchestrator::video::UpdateGeneratedVideoRecordInput;

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
        .orchestrator
        .generated_videos_for_chapter(project_id, chapter_id)
        .await
        .map_err(app_error_message)?;
    Ok(rows.iter().map(video_to_summary).collect())
}

pub async fn toggle_video_favorite(
    state: &AppState,
    user_id: i64,
    video_id: i64,
) -> Result<GeneratedVideoSummary, String> {
    let video = required_generated_video(state, video_id).await?;
    access::verify_project_editor_string(state, user_id, video.project_id).await?;
    let row = state
        .orchestrator
        .toggle_generated_video_favorite(video_id)
        .await
        .map_err(app_error_message)?;
    Ok(video_to_summary(&row))
}

pub async fn update_video(
    state: &AppState,
    user_id: i64,
    video_id: i64,
    input: UpdateGeneratedVideoInput,
) -> Result<GeneratedVideoSummary, String> {
    let existing = required_generated_video(state, video_id).await?;
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
                .orchestrator
                .generated_videos_for_chapter(existing.project_id, chapter_id)
                .await
                .map_err(app_error_message)?;
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
        .orchestrator
        .update_generated_video_record(
            video_id,
            UpdateGeneratedVideoRecordInput {
                prompt: input.prompt,
                negative_prompt: input.negative_prompt,
                clear_negative_prompt: input.clear_negative_prompt,
                metadata_json,
                clear_metadata_json,
                is_favorite: input.is_favorite,
            },
        )
        .await
        .map_err(app_error_message)?;
    Ok(video_to_summary(&row))
}

pub(super) async fn required_generated_video(
    state: &AppState,
    video_id: i64,
) -> Result<generated_video::Model, String> {
    state
        .orchestrator
        .generated_video_by_id(video_id)
        .await
        .map_err(app_error_message)?
        .ok_or_else(|| "视频不存在".to_string())
}
