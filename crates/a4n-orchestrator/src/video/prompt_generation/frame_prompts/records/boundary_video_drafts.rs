use std::collections::HashMap;

use a4n_core::error::AppError;
use a4n_core::models::chapter::chapter_video_prompt;
use serde_json::json;

use super::super::super::super::FRAME_SEGMENT_PLAN_CONTRACT_VERSION;
use super::super::super::super::prompt_json::{
    VideoFramePrompt, VideoPromptShot, VideoSegmentPlan,
};
use super::super::super::super::prompt_store::{NewChapterVideoPrompt, PromptWorkflowMode};
use super::super::VideoUnitPlan;
use a4n_core::prompt_language::with_prompt_output_language_metadata;

use super::frame_drafts::{VideoPromptDraftCommonInput, video_prompt_shot_draft};
use super::rules::compact_shot_metadata;

pub(in crate::video) struct VideoPromptDraftsInput<'a> {
    pub common: VideoPromptDraftCommonInput<'a>,
    pub segment_by_number: &'a HashMap<i32, &'a VideoSegmentPlan>,
    pub video_unit_by_number: &'a HashMap<i32, VideoUnitPlan>,
    pub frame_by_number: &'a HashMap<i32, VideoFramePrompt>,
    pub video_prompts: &'a [VideoPromptShot],
}

pub(in crate::video) fn video_prompt_drafts(
    input: VideoPromptDraftsInput<'_>,
) -> Result<Vec<NewChapterVideoPrompt>, AppError> {
    let VideoPromptDraftsInput {
        common,
        segment_by_number,
        video_unit_by_number,
        frame_by_number,
        video_prompts,
    } = input;
    video_prompts
        .iter()
        .map(|prompt| {
            let segment = segment_by_number.get(&prompt.shot_number).ok_or_else(|| {
                AppError::InvalidArgument(format!(
                    "视频提示词 #{} 缺少对应片段规划",
                    prompt.shot_number
                ))
            })?;
            let video_unit = video_unit_by_number
                .get(&prompt.shot_number)
                .ok_or_else(|| {
                    AppError::InvalidArgument(format!(
                        "视频提示词 #{} 缺少对应视频单元规划",
                        prompt.shot_number
                    ))
                })?;
            let start_frame_number = video_unit.start_frame_number.ok_or_else(|| {
                AppError::InvalidArgument(format!(
                    "视频提示词 #{} 对应视频单元缺少起始边界帧编号",
                    prompt.shot_number
                ))
            })?;
            let end_frame_number = video_unit.end_frame_number.ok_or_else(|| {
                AppError::InvalidArgument(format!(
                    "视频提示词 #{} 对应视频单元缺少结束边界帧编号",
                    prompt.shot_number
                ))
            })?;
            let start_boundary_frame =
                frame_by_number.get(&start_frame_number).ok_or_else(|| {
                    AppError::InvalidArgument(format!(
                        "视频提示词 #{} 缺少起始边界帧 #{}",
                        prompt.shot_number, start_frame_number
                    ))
                })?;
            let end_boundary_frame = frame_by_number.get(&end_frame_number).ok_or_else(|| {
                AppError::InvalidArgument(format!(
                    "视频提示词 #{} 缺少结束边界帧 #{}",
                    prompt.shot_number, end_frame_number
                ))
            })?;
            Ok(video_prompt_shot_draft(
                common,
                PromptWorkflowMode::BoundaryFrames,
                prompt,
                with_prompt_output_language_metadata(
                    json!({
                        "contract_version": FRAME_SEGMENT_PLAN_CONTRACT_VERSION,
                        "shot": compact_shot_metadata(prompt),
                        "video_unit": video_unit,
                        "segment": *segment,
                        "start_boundary_frame": start_boundary_frame,
                        "end_boundary_frame": end_boundary_frame,
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
                        "source_prompt_type":
                            chapter_video_prompt::PROMPT_TYPE_FRAME_SEGMENT_PLAN,
                        "source": common.source_metadata.clone()
                    }),
                    common.prompt_output_language,
                ),
            ))
        })
        .collect()
}
