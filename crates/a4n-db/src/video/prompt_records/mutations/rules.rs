use crate::video::VideoDbError;
use a4n_core::models::chapter::chapter_video_prompt;

pub(in crate::video::prompt_records) fn validate_chapter_video_prompt_duration(
    duration_secs: i32,
) -> Result<(), VideoDbError> {
    if !(1..=60).contains(&duration_secs) {
        return Err(VideoDbError::InvalidArgument(
            "视频提示词 duration_secs 必须在 1 到 60 秒之间".to_string(),
        ));
    }
    Ok(())
}

pub(in crate::video::prompt_records) fn ensure_chapter_video_prompt_not_running(
    prompt: &chapter_video_prompt::Model,
    operation: &str,
) -> Result<(), VideoDbError> {
    if prompt.status == "running" {
        return Err(VideoDbError::InvalidState {
            current: prompt.status.clone(),
            operation: operation.to_string(),
        });
    }
    Ok(())
}
