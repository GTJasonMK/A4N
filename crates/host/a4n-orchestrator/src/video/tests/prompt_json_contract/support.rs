use super::super::prompts::json::{
    VideoFramePrompt, VideoFramePromptDefaults, parse_storyboard_frame_prompts_with_defaults,
    parse_video_frame_prompts_with_defaults,
};
use a4n_error::AppError;
use a4n_domain::prompt_language::PromptOutputLanguage;

fn parse_video_frame_prompts(raw: &str) -> Result<Vec<VideoFramePrompt>, AppError> {
    let value: serde_json::Value = serde_json::from_str(raw)?;
    let frame_count = value
        .get("frames")
        .and_then(serde_json::Value::as_array)
        .map(Vec::len)
        .unwrap_or_default();
    let defaults = (0..frame_count)
        .map(|index| VideoFramePromptDefaults {
            frame_number: (index + 1) as i32,
            duration_secs: 6,
            aspect_ratio: "16:9",
            segment_numbers: vec![1],
            reference_image_ids: vec![3],
        })
        .collect::<Vec<_>>();
    parse_video_frame_prompts_with_defaults(raw, &defaults, PromptOutputLanguage::ZhCn)
}
