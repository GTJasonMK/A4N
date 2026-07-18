use super::super::contracts::{ASSEMBLY_INSERT_VIDEO_TYPE, AssemblyClip, CHAPTER_VIDEO_TYPE};
use super::metadata::{
    assembly_clip_display_shot_number, assembly_clip_insert_after_video_id,
    assembly_clip_sort_order,
};
use crate::AppState;
use crate::file_utils::canonicalize_data_file_path;
use a4n_core::models::ai::generated_video;

pub(in crate::actions::video_records) fn assembly_clips(
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

pub(in crate::actions::video_records) fn is_assembly_clip_video_type(video_type: &str) -> bool {
    matches!(video_type, CHAPTER_VIDEO_TYPE | ASSEMBLY_INSERT_VIDEO_TYPE)
}
