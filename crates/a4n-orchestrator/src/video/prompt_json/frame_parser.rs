use std::collections::HashSet;

use a4n_core::error::AppError;

use a4n_core::prompt_language::PromptOutputLanguage;

use super::contracts::{
    StaticVideoFramePromptInput, StaticVideoFramePromptResponse, StoryboardVideoFramePromptInput,
    StoryboardVideoFramePromptResponse, VideoFramePrompt, VideoFramePromptDefaults,
};
use super::{parse_strict_json_response, validate_frame_prompt};

pub(in crate::video) fn parse_video_frame_prompts_with_defaults(
    raw: &str,
    defaults: &[VideoFramePromptDefaults<'_>],
    prompt_output_language: PromptOutputLanguage,
) -> Result<Vec<VideoFramePrompt>, AppError> {
    let response: StaticVideoFramePromptResponse =
        parse_strict_json_response(raw, "视频首尾帧提示词")?;
    let frames = response
        .frames
        .into_iter()
        .map(ModelVideoFramePrompt::from)
        .collect();
    materialize_video_frame_prompts(frames, defaults, prompt_output_language, "视频首尾帧提示词")
}

pub(in crate::video) fn parse_storyboard_frame_prompts_with_defaults(
    raw: &str,
    defaults: &[VideoFramePromptDefaults<'_>],
    prompt_output_language: PromptOutputLanguage,
) -> Result<Vec<VideoFramePrompt>, AppError> {
    let response: StoryboardVideoFramePromptResponse =
        parse_strict_json_response(raw, "视频分镜提示词")?;
    let frames = response
        .frames
        .into_iter()
        .map(ModelVideoFramePrompt::from)
        .collect();
    materialize_video_frame_prompts(frames, defaults, prompt_output_language, "视频分镜提示词")
}

struct ModelVideoFramePrompt {
    title: String,
    scene_summary: String,
    storyboard_panels: Vec<super::contracts::StoryboardPanel>,
    frame_prompt: String,
    blocking_state: serde_json::Value,
    lighting_state: serde_json::Value,
    negative_prompt: String,
}

impl From<StaticVideoFramePromptInput> for ModelVideoFramePrompt {
    fn from(value: StaticVideoFramePromptInput) -> Self {
        Self {
            title: value.title,
            scene_summary: value.scene_summary,
            storyboard_panels: Vec::new(),
            frame_prompt: value.frame_prompt,
            blocking_state: value.blocking_state,
            lighting_state: value.lighting_state,
            negative_prompt: value.negative_prompt,
        }
    }
}

impl From<StoryboardVideoFramePromptInput> for ModelVideoFramePrompt {
    fn from(value: StoryboardVideoFramePromptInput) -> Self {
        Self {
            title: value.title,
            scene_summary: value.scene_summary,
            storyboard_panels: value.storyboard_panels,
            frame_prompt: value.frame_prompt,
            blocking_state: value.blocking_state,
            lighting_state: value.lighting_state,
            negative_prompt: value.negative_prompt,
        }
    }
}

fn materialize_video_frame_prompts(
    model_frames: Vec<ModelVideoFramePrompt>,
    defaults: &[VideoFramePromptDefaults<'_>],
    prompt_output_language: PromptOutputLanguage,
    label: &str,
) -> Result<Vec<VideoFramePrompt>, AppError> {
    if model_frames.is_empty() {
        return Err(AppError::InvalidArgument(format!(
            "{label}响应必须包含至少一个 frames 条目"
        )));
    }
    if defaults.len() != model_frames.len() {
        return Err(AppError::InvalidArgument(format!(
            "{label}默认字段数量必须等于模型返回帧数：默认 {} 条，实际 {} 条",
            defaults.len(),
            model_frames.len()
        )));
    }
    let mut seen_numbers = HashSet::new();
    let mut frames = Vec::with_capacity(model_frames.len());
    for (index, frame) in model_frames.into_iter().enumerate() {
        let defaults = &defaults[index];
        let frame_number = defaults.frame_number;
        let duration_secs = defaults.duration_secs;
        let aspect_ratio = defaults.aspect_ratio.to_string();
        let reference_image_ids = defaults.reference_image_ids.clone();
        let segment_numbers = defaults.segment_numbers.clone();
        let frame = VideoFramePrompt {
            frame_number,
            title: frame.title,
            scene_summary: frame.scene_summary,
            storyboard_panels: frame.storyboard_panels,
            frame_prompt: frame.frame_prompt,
            blocking_state: frame.blocking_state,
            lighting_state: frame.lighting_state,
            negative_prompt: frame.negative_prompt,
            duration_secs,
            aspect_ratio,
            reference_image_ids,
            segment_numbers,
        };
        let frame = validate_frame_prompt(frame, prompt_output_language)?;
        if !seen_numbers.insert(frame.frame_number) {
            return Err(AppError::InvalidArgument(format!(
                "{label} frame_number {} 重复",
                frame.frame_number
            )));
        }
        frames.push(frame);
    }
    Ok(frames)
}
