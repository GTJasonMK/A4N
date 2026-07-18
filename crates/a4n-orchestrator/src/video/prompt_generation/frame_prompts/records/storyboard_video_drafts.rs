use std::collections::HashMap;

use a4n_core::error::AppError;
use serde_json::json;

use super::super::super::super::STORYBOARD_VIDEO_PROMPT_CONTRACT_VERSION;
use super::super::super::super::prompt_json::{
    VideoFramePrompt, VideoPromptShot, VideoSegmentPlan,
};
use super::super::super::super::prompt_store::{
    NewChapterVideoPrompt, NewChapterVideoPromptInput, PromptWorkflowMode, new_prompt_draft,
};
use super::super::VideoUnitPlan;
use a4n_core::prompt_language::with_prompt_output_language_metadata;

use super::frame_drafts::VideoPromptDraftCommonInput;
use super::rules::{compact_shot_metadata, optional_prompt_text};

pub(in crate::video) struct StoryboardVideoPromptDraftsInput<'a> {
    pub common: VideoPromptDraftCommonInput<'a>,
    pub segment_by_number: &'a HashMap<i32, &'a VideoSegmentPlan>,
    pub video_unit_by_number: &'a HashMap<i32, VideoUnitPlan>,
    pub storyboard_by_number: &'a HashMap<i32, VideoFramePrompt>,
    pub video_prompts: &'a [VideoPromptShot],
}

pub(in crate::video) fn storyboard_video_prompt_drafts(
    input: StoryboardVideoPromptDraftsInput<'_>,
) -> Result<Vec<NewChapterVideoPrompt>, AppError> {
    let StoryboardVideoPromptDraftsInput {
        common,
        segment_by_number,
        video_unit_by_number,
        storyboard_by_number,
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
                    "分镜视频提示词 #{} 缺少对应片段规划",
                    prompt.shot_number
                ))
            })?;
            let video_unit = video_unit_by_number
                .get(&prompt.shot_number)
                .ok_or_else(|| {
                    AppError::InvalidArgument(format!(
                        "分镜视频提示词 #{} 缺少对应视频单元规划",
                        prompt.shot_number
                    ))
                })?;
            let storyboard = storyboard_by_number
                .get(&prompt.shot_number)
                .ok_or_else(|| {
                    AppError::InvalidArgument(format!(
                        "分镜视频提示词 #{} 缺少对应分镜提示词",
                        prompt.shot_number
                    ))
                })?;
            Ok(new_prompt_draft(NewChapterVideoPromptInput {
                project_id,
                chapter_id,
                workflow_mode: PromptWorkflowMode::Storyboard,
                style: Some(style),
                prompt_type: "video_prompt",
                shot_number: prompt.shot_number,
                title: Some(&prompt.title),
                scene_summary: Some(&prompt.scene_summary),
                visual_prompt: &prompt.visual_prompt,
                motion_prompt: Some(&prompt.motion_prompt),
                negative_prompt: optional_prompt_text(&prompt.negative_prompt),
                duration_secs: prompt.duration_secs,
                aspect_ratio: Some(&prompt.aspect_ratio),
                reference_image_ids_json: Some(json!(prompt.reference_image_ids)),
                metadata_json: Some(with_prompt_output_language_metadata(
                    json!({
                        "contract_version": STORYBOARD_VIDEO_PROMPT_CONTRACT_VERSION,
                        "shot": compact_shot_metadata(prompt),
                        "video_unit": video_unit,
                        "segment": *segment,
                        "storyboard_frame": storyboard,
                        "storyboard": {
                            "prompt_number": prompt.shot_number,
                            "image_id": prompt.reference_image_ids.first().copied()
                        },
                        "available_assets": available_assets.clone(),
                        "generation_asset_fingerprint": generation_asset_fingerprint.clone(),
                        "video_execution_identity": execution_identity.clone(),
                        "self_contained_context": prompt.self_contained_context.clone(),
                        "start_state": prompt.start_state.clone(),
                        "end_state": prompt.end_state.clone(),
                        "transition_in": prompt.transition_in.clone(),
                        "blocking_plan": prompt.blocking_plan.clone(),
                        "lighting_plan": prompt.lighting_plan.clone(),
                        "emotion_action_beats": prompt.emotion_action_beats.clone(),
                        "dialogue_beats": prompt.dialogue_beats.clone(),
                        "cinematography_plan": prompt.cinematography_plan.clone(),
                        "temporal_beats": prompt.temporal_beats.clone(),
                        "reference_binding": prompt.reference_binding.clone(),
                        "continuity_check": prompt.continuity_check.clone(),
                        "source_prompt_type": "frame_segment_plan",
                        "source": source_metadata.clone()
                    }),
                    prompt_output_language,
                )),
            }))
        })
        .collect()
}
