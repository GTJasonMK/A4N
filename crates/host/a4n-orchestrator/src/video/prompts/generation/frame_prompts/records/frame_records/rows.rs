use a4n_db_models::models::chapter::chapter_video_prompt;
use a4n_error::AppError;

use crate::video::prompts::{json::VideoFramePrompt, records::reusable_rows_are_complete};

pub(in crate::video) fn complete_reusable_rows(
    rows: Vec<chapter_video_prompt::Model>,
    expected_count: usize,
) -> Option<Vec<chapter_video_prompt::Model>> {
    reusable_rows_are_complete(&rows, expected_count).then_some(rows)
}

pub(in crate::video) fn initial_first_frame_rows(
    rows: Vec<chapter_video_prompt::Model>,
) -> Vec<chapter_video_prompt::Model> {
    rows.into_iter()
        .filter(|row| row.shot_number == 1)
        .collect()
}

pub(in crate::video) fn frame_prompts_from_rows(
    rows: &[chapter_video_prompt::Model],
) -> Result<Vec<VideoFramePrompt>, AppError> {
    saved_frame_prompts_from_rows(rows, "boundary_frame", "边界帧提示词")
}

pub(in crate::video) fn first_frame_prompts_from_rows(
    rows: &[chapter_video_prompt::Model],
) -> Result<Vec<VideoFramePrompt>, AppError> {
    saved_frame_prompts_from_rows(rows, "first_frame", "首帧提示词")
}

pub(in crate::video) fn storyboard_prompts_from_rows(
    rows: &[chapter_video_prompt::Model],
) -> Result<Vec<VideoFramePrompt>, AppError> {
    saved_frame_prompts_from_rows(rows, "storyboard_frame", "分镜提示词")
}

fn saved_frame_prompts_from_rows(
    rows: &[chapter_video_prompt::Model],
    metadata_key: &str,
    label: &str,
) -> Result<Vec<VideoFramePrompt>, AppError> {
    rows.iter()
        .map(|row| {
            let value = row
                .metadata_json
                .as_ref()
                .and_then(|metadata| metadata.get(metadata_key))
                .cloned()
                .ok_or_else(|| {
                    AppError::InvalidArgument(format!(
                        "已保存{label} #{} 缺少 {metadata_key} 元数据",
                        row.shot_number,
                    ))
                })?;
            serde_json::from_value::<VideoFramePrompt>(value).map_err(|err| {
                AppError::InvalidArgument(format!(
                    "已保存{label} #{} 的 {metadata_key} 元数据无效：{}",
                    row.shot_number, err,
                ))
            })
        })
        .collect()
}
