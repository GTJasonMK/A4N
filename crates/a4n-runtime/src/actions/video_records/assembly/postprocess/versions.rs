use super::super::contracts::{ASSEMBLY_POSTPROCESS_EDIT_VIDEO_TYPE, PostprocessEditMetadata};
use super::metadata::parse_postprocess_edit_metadata;
use crate::AppState;
use crate::actions::app_error_message;
use a4n_core::models::ai::generated_video;

pub(in crate::actions::video_records::assembly) async fn latest_postprocess_version(
    state: &AppState,
    push: &generated_video::Model,
) -> Result<generated_video::Model, String> {
    let chapter_id = push
        .chapter_id
        .ok_or_else(|| format!("推送批次 #{} 缺少章节绑定", push.id))?;
    let mut versions = Vec::new();
    for row in state
        .orchestrator
        .generated_videos_for_chapter(push.project_id, chapter_id)
        .await
        .map_err(app_error_message)?
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
