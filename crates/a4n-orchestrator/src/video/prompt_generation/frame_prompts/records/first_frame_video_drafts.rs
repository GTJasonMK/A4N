use std::collections::HashMap;

use a4n_error::AppError;
use serde_json::{Value, json};

use super::super::super::super::FIRST_FRAME_CHAIN_CONTRACT_VERSION;
use super::super::super::super::prompt_json::{
    VideoFramePrompt, VideoPromptShot, VideoSegmentPlan,
};
use super::super::super::super::prompt_store::{NewChapterVideoPrompt, PromptWorkflowMode};
use super::super::VideoUnitPlan;
use a4n_domain::prompt_language::with_prompt_output_language_metadata;

use super::frame_drafts::{VideoPromptDraftCommonInput, video_prompt_shot_draft};
use super::rules::compact_shot_metadata;

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
                match first_frame_by_number.get(&1) {
                    Some(frame) => json!(frame),
                    None if segment.director_constraints.is_some() => Value::Null,
                    None => return Err(AppError::InvalidArgument("首帧串联视频提示词缺少初始首帧 #1".to_string())),
                }
            } else {
                Value::Null
            };
            Ok(video_prompt_shot_draft(
                common,
                PromptWorkflowMode::FirstFrameChain,
                prompt,
                with_prompt_output_language_metadata(
                    json!({
                        "contract_version": FIRST_FRAME_CHAIN_CONTRACT_VERSION,
                        "shot": compact_shot_metadata(prompt),
                        "video_unit": video_unit,
                        "segment": *segment,
                        "start_first_frame": start_first_frame,
                        "available_assets": common.available_assets.clone(),
                        "generation_asset_fingerprint": common.generation_asset_fingerprint.clone(),
                        "video_execution_identity": common.execution_identity.clone(),
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
                            "start_source": if segment.director_constraints.is_some() {
                                "director_timeline"
                            } else if prompt.shot_number == 1 {
                                "generated_first_frame"
                            } else {
                                "previous_video_tail_frame"
                            },
                            "first_frame_prompt_number": if prompt.shot_number == 1 && segment.director_constraints.is_none() {
                                Value::from(1)
                            } else {
                                Value::Null
                            },
                            "previous_video_shot_number": if prompt.shot_number > 1 && segment.director_constraints.is_none() {
                                Value::from(prompt.shot_number - 1)
                            } else {
                                Value::Null
                            }
                        },
                        "source": common.source_metadata.clone()
                    }),
                    common.prompt_output_language,
                ),
            ))
        })
        .collect()
}
