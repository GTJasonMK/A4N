use std::collections::HashMap;

use a4n_core::error::AppError;
use serde_json::{Value, json};

use super::super::super::super::FIRST_FRAME_CHAIN_CONTRACT_VERSION;
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

pub(in crate::video) struct FirstFrameVideoPromptDraftsInput<'a> {
    pub common: VideoPromptDraftCommonInput<'a>,
    pub source_prompt_type: &'a str,
    pub segment_by_number: &'a HashMap<i32, &'a VideoSegmentPlan>,
    pub video_unit_by_number: &'a HashMap<i32, VideoUnitPlan>,
    pub first_frame_by_number: &'a HashMap<i32, VideoFramePrompt>,
    pub video_prompts: &'a [VideoPromptShot],
}

pub(in crate::video) fn first_frame_video_prompt_drafts(
    input: FirstFrameVideoPromptDraftsInput<'_>,
) -> Result<Vec<NewChapterVideoPrompt>, AppError> {
    let FirstFrameVideoPromptDraftsInput {
        common,
        source_prompt_type,
        segment_by_number,
        video_unit_by_number,
        first_frame_by_number,
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
    let generated_first_frame = first_frame_by_number.get(&1).ok_or_else(|| {
        AppError::InvalidArgument("首帧串联视频提示词缺少初始首帧 #1".to_string())
    })?;
    video_prompts
        .iter()
        .map(|prompt| {
            let segment = segment_by_number.get(&prompt.shot_number).ok_or_else(|| {
                AppError::InvalidArgument(format!(
                    "首帧串联视频提示词 #{} 缺少对应片段规划",
                    prompt.shot_number
                ))
            })?;
            let video_unit = video_unit_by_number
                .get(&prompt.shot_number)
                .ok_or_else(|| {
                    AppError::InvalidArgument(format!(
                        "首帧串联视频提示词 #{} 缺少对应视频单元规划",
                        prompt.shot_number
                    ))
                })?;
            let start_first_frame = if prompt.shot_number == 1 {
                json!(generated_first_frame)
            } else {
                Value::Null
            };
            Ok(new_prompt_draft(NewChapterVideoPromptInput {
                project_id,
                chapter_id,
                workflow_mode: PromptWorkflowMode::FirstFrameChain,
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
                        "contract_version": FIRST_FRAME_CHAIN_CONTRACT_VERSION,
                        "shot": compact_shot_metadata(prompt),
                        "video_unit": video_unit,
                        "segment": *segment,
                        "start_first_frame": start_first_frame,
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
                        "source_prompt_type": source_prompt_type,
                        "first_frame_chain": {
                            "start_source": if prompt.shot_number == 1 {
                                "generated_first_frame"
                            } else {
                                "previous_video_tail_frame"
                            },
                            "first_frame_prompt_number": if prompt.shot_number == 1 {
                                Value::from(1)
                            } else {
                                Value::Null
                            },
                            "previous_video_shot_number": if prompt.shot_number > 1 {
                                Value::from(prompt.shot_number - 1)
                            } else {
                                Value::Null
                            }
                        },
                        "source": source_metadata.clone()
                    }),
                    prompt_output_language,
                )),
            }))
        })
        .collect()
}
