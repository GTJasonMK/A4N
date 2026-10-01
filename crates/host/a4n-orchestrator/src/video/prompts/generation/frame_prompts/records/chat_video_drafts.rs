use std::collections::HashMap;

use a4n_domain::video_prompt::chapter_video_prompt;
use a4n_error::AppError;
use serde_json::{Value, json};

use super::super::super::super::super::CHAT_VIDEO_CONTRACT_VERSION;
use super::super::super::super::json::{ChatVideoPromptShot, VideoSegmentPlan};
use super::super::super::super::store::{
    NewChapterVideoPrompt, NewChapterVideoPromptInput, PromptWorkflowMode, new_prompt_draft,
};
use super::super::VideoUnitPlan;
use a4n_domain::prompt_language::with_prompt_output_language_metadata;

use super::frame_drafts::VideoPromptDraftCommonInput;
use super::rules::optional_prompt_text;

pub(in crate::video) struct ChatVideoPromptDraftsInput<'a> {
    pub common: VideoPromptDraftCommonInput<'a>,
    pub source_prompt_type: &'a str,
    pub segment_by_number: &'a HashMap<i32, &'a VideoSegmentPlan>,
    pub video_unit_by_number: &'a HashMap<i32, VideoUnitPlan>,
    pub generation_reference_ids: &'a Value,
    pub video_prompts: &'a [ChatVideoPromptShot],
}

pub(in crate::video) fn chat_video_prompt_drafts(
    input: ChatVideoPromptDraftsInput<'_>,
) -> Result<Vec<NewChapterVideoPrompt>, AppError> {
    let ChatVideoPromptDraftsInput {
        common,
        source_prompt_type,
        segment_by_number,
        video_unit_by_number,
        generation_reference_ids,
        video_prompts,
    } = input;
    let VideoPromptDraftCommonInput {
        project_id,
        chapter_id,
        style,
        prompt_output_language,
        source_metadata,
        available_assets,
        generation_asset_fingerprint,
        execution_identity,
    } = common;
    video_prompts
        .iter()
        .map(|prompt| {
            let segment = segment_by_number.get(&prompt.shot_number).ok_or_else(|| {
                AppError::InvalidArgument(format!(
                    "Chat 视频提示词 #{} 缺少对应片段规划",
                    prompt.shot_number
                ))
            })?;
            let video_unit = video_unit_by_number
                .get(&prompt.shot_number)
                .ok_or_else(|| {
                    AppError::InvalidArgument(format!(
                        "Chat 视频提示词 #{} 缺少对应视频单元规划",
                        prompt.shot_number
                    ))
                })?;
            Ok(new_prompt_draft(NewChapterVideoPromptInput {
                project_id,
                chapter_id,
                workflow_mode: PromptWorkflowMode::ChatVideo,
                style: Some(style),
                prompt_type: chapter_video_prompt::PROMPT_TYPE_VIDEO,
                shot_number: prompt.shot_number,
                title: Some(&prompt.title),
                scene_summary: Some(&prompt.scene_summary),
                visual_prompt: &prompt.chat_prompt,
                motion_prompt: None,
                negative_prompt: optional_prompt_text(&prompt.negative_prompt),
                duration_secs: prompt.duration_secs,
                aspect_ratio: Some(&prompt.aspect_ratio),
                reference_image_ids_json: Some(json!(prompt.reference_image_ids)),
                metadata_json: Some(with_prompt_output_language_metadata(
                    json!({
                        "contract_version": CHAT_VIDEO_CONTRACT_VERSION,
                        "chat_video": prompt.chat_video.clone(),
                        "video_unit": video_unit,
                        "segment": *segment,
                        "available_assets": available_assets.clone(),
                        "generation_reference_image_ids": generation_reference_ids.clone(),
                        "generation_asset_fingerprint": generation_asset_fingerprint.clone(),
                        "video_execution_identity": execution_identity.clone(),
                        "source_prompt_type": source_prompt_type,
                        "source": source_metadata.clone()
                    }),
                    prompt_output_language,
                )),
            }))
        })
        .collect()
}
