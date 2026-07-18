use super::super::contracts::{AssemblyClip, AssemblyPushRow};
use super::metadata::assembly_push_rows;
use super::selection::{active_assembly_push_source_ids, ensure_active_pushes_match_clip_prefix};
use crate::AppState;
use crate::actions::app_error_message;
use std::collections::HashSet;

pub(in crate::actions::video_records) async fn pushed_source_ids_for_chapter(
    state: &AppState,
    project_id: i64,
    chapter_id: i64,
    workflow_mode: &str,
) -> Result<HashSet<i64>, String> {
    let rows = state
        .orchestrator
        .generated_videos_for_chapter(project_id, chapter_id)
        .await
        .map_err(app_error_message)?;
    Ok(
        active_assembly_push_source_ids(&assembly_push_rows(&rows, workflow_mode)?)
            .into_iter()
            .collect(),
    )
}

pub(in crate::actions::video_records) fn ensure_insert_position_not_inside_pushed_prefix(
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
    let pushed_last_index = active_source_ids.len() - 1;
    if anchor_index < pushed_last_index {
        return Err("不能在已推送成片片段中间插入衔接片段".to_string());
    }
    Ok(())
}
