use std::collections::HashMap;

use a4n_core::error::AppError;
use serde_json::json;

use super::super::super::super::FRAME_SEGMENT_PLAN_CONTRACT_VERSION;
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
            Ok(new_prompt_draft(NewChapterVideoPromptInput {
                project_id,
                chapter_id,
                workflow_mode: PromptWorkflowMode::BoundaryFrames,
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
                        "contract_version": FRAME_SEGMENT_PLAN_CONTRACT_VERSION,
                        "shot": compact_shot_metadata(prompt),
                        "video_unit": video_unit,
                        "segment": *segment,
                        "start_boundary_frame": start_boundary_frame,
                        "end_boundary_frame": end_boundary_frame,
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
