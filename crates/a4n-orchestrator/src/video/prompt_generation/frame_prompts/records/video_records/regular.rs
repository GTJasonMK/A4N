use std::collections::BTreeMap;

use a4n_db_models::models::chapter::chapter_video_prompt;
use a4n_error::AppError;

use super::super::super::super::super::prompt_json::VideoPromptShot;
use super::metadata::{
    SavedVideoPromptMetadata, required_row_aspect_ratio, required_row_text, row_metadata,
    row_reference_image_ids,
};

pub(in crate::video) fn reusable_boundary_video_prompts_from_rows(
    rows: &[chapter_video_prompt::Model],
) -> Result<BTreeMap<i32, VideoPromptShot>, AppError> {
    reusable_video_prompts_from_rows(rows, "边界帧视频提示词")
}

pub(in crate::video) fn reusable_first_frame_video_prompts_from_rows(
    rows: &[chapter_video_prompt::Model],
) -> Result<BTreeMap<i32, VideoPromptShot>, AppError> {
    reusable_video_prompts_from_rows(rows, "首帧串联视频提示词")
}

fn reusable_video_prompts_from_rows(
    rows: &[chapter_video_prompt::Model],
    label: &str,
) -> Result<BTreeMap<i32, VideoPromptShot>, AppError> {
    let mut by_shot = BTreeMap::<i32, VideoPromptShot>::new();
    for row in rows {
        let prompt = reusable_video_prompt_from_row(row, label)?;
        by_shot.entry(row.shot_number).or_insert(prompt);
    }
    Ok(by_shot)
}

fn reusable_video_prompt_from_row(
    row: &chapter_video_prompt::Model,
    label: &str,
) -> Result<VideoPromptShot, AppError> {
    let metadata = row_metadata::<SavedVideoPromptMetadata>(row, label)?;
    let motion_prompt = row.motion_prompt.clone().ok_or_else(|| {
        AppError::InvalidArgument(format!(
            "已保存{label} #{} 缺少 motion_prompt",
            row.shot_number
        ))
    })?;
    let aspect_ratio = required_row_aspect_ratio(row, label)?;
    let reference_image_ids = row_reference_image_ids(row, label)?;
    Ok(VideoPromptShot {
        shot_number: row.shot_number,
        title: required_row_text(&row.title, row, label, "title")?,
        scene_summary: required_row_text(&row.scene_summary, row, label, "scene_summary")?,
        self_contained_context: metadata.self_contained_context,
        start_state: metadata.start_state,
        end_state: metadata.end_state,
        transition_in: metadata.transition_in,
        visual_prompt: row.visual_prompt.clone(),
        motion_prompt,
        negative_prompt: row.negative_prompt.clone().unwrap_or_default(),
        blocking_plan: metadata.blocking_plan,
        lighting_plan: metadata.lighting_plan,
        emotion_action_beats: metadata.emotion_action_beats,
        dialogue_beats: metadata.dialogue_beats,
        cinematography_plan: metadata.cinematography_plan,
        temporal_beats: metadata.temporal_beats,
        reference_binding: metadata.reference_binding,
        continuity_check: metadata.continuity_check,
        duration_secs: row.duration_secs,
        aspect_ratio,
        reference_image_ids,
    })
}
