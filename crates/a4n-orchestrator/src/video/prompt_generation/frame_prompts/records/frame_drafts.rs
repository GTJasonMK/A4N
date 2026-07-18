use a4n_core::error::AppError;
use a4n_core::models::chapter::chapter_video_prompt;
use serde_json::{Value, json};

use super::super::super::super::prompt_json::VideoFramePrompt;
use super::super::super::super::prompt_store::{
    NewChapterVideoPrompt, NewChapterVideoPromptInput, PromptWorkflowMode, new_prompt_draft,
};
use super::super::super::super::{
    BOUNDARY_FRAME_PROMPT_CONTRACT_VERSION, FIRST_FRAME_CHAIN_CONTRACT_VERSION,
    PromptOutputLanguage, STORYBOARD_PROMPT_CONTRACT_VERSION,
};
use super::super::VideoUnitPlan;
use a4n_core::prompt_language::with_prompt_output_language_metadata;

pub(in crate::video) fn prompt_row_draft(
    row: &chapter_video_prompt::Model,
) -> Result<NewChapterVideoPrompt, AppError> {
    Ok(new_prompt_draft(NewChapterVideoPromptInput {
        project_id: row.project_id,
        chapter_id: row.chapter_id,
        workflow_mode: PromptWorkflowMode::from_row(row)?,
        style: row.style.as_deref(),
        prompt_type: &row.prompt_type,
        shot_number: row.shot_number,
        title: row.title.as_deref(),
        scene_summary: row.scene_summary.as_deref(),
        visual_prompt: &row.visual_prompt,
        motion_prompt: row.motion_prompt.as_deref(),
        negative_prompt: row.negative_prompt.as_deref(),
        duration_secs: row.duration_secs,
        aspect_ratio: row.aspect_ratio.as_deref(),
        reference_image_ids_json: row.reference_image_ids_json.clone(),
        metadata_json: row.metadata_json.clone(),
    }))
}

#[derive(Clone, Copy)]
pub(in crate::video) struct VideoPromptDraftCommonInput<'a> {
    pub project_id: i64,
    pub chapter_id: i64,
    pub style: &'a str,
    pub prompt_output_language: PromptOutputLanguage,
    pub source_metadata: &'a Value,
    pub available_assets: &'a Value,
    pub generation_asset_fingerprint: &'a Value,
    pub execution_identity: &'a Value,
}

pub(in crate::video) struct FramePromptDraftsInput<'a> {
    pub common: VideoPromptDraftCommonInput<'a>,
    pub workflow_mode: PromptWorkflowMode,
    pub prompt_type: &'a str,
    pub source_prompt_type: &'a str,
    pub frame_prompts: &'a [VideoFramePrompt],
}

pub(in crate::video) fn frame_prompt_drafts(
    input: FramePromptDraftsInput<'_>,
) -> Vec<NewChapterVideoPrompt> {
    let FramePromptDraftsInput {
        common,
        workflow_mode,
        prompt_type,
        source_prompt_type,
        frame_prompts,
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
    frame_prompts
        .iter()
        .map(|frame| {
            let metadata = with_prompt_output_language_metadata(
                json!({
                    "contract_version": BOUNDARY_FRAME_PROMPT_CONTRACT_VERSION,
                    "boundary_frame": frame.clone(),
                    "video_frame_boundary": {
                        "source_segment_numbers": frame.segment_numbers,
                        "shared_between_units": frame.segment_numbers.len() > 1
                    },
                    "reference_slots": super::super::reference_slots_for_frame(
                        frame.reference_image_ids.len(),
                        None,
                    ),
                    "blocking_state": frame.blocking_state.clone(),
                    "lighting_state": frame.lighting_state.clone(),
                    "available_assets": available_assets.clone(),
                    "generation_asset_fingerprint": generation_asset_fingerprint.clone(),
                    "video_execution_identity": execution_identity.clone(),
                    "source_prompt_type": source_prompt_type,
                    "source": source_metadata.clone()
                }),
                prompt_output_language,
            );
            new_prompt_draft(NewChapterVideoPromptInput {
                project_id,
                chapter_id,
                workflow_mode,
                style: Some(style),
                prompt_type,
                shot_number: frame.frame_number,
                title: Some(&frame.title),
                scene_summary: Some(&frame.scene_summary),
                visual_prompt: &frame.frame_prompt,
                motion_prompt: None,
                negative_prompt: Some(&frame.negative_prompt),
                duration_secs: frame.duration_secs,
                aspect_ratio: Some(&frame.aspect_ratio),
                reference_image_ids_json: Some(json!(frame.reference_image_ids)),
                metadata_json: Some(metadata),
            })
        })
        .collect()
}

pub(in crate::video) struct FirstFramePromptDraftsInput<'a> {
    pub common: VideoPromptDraftCommonInput<'a>,
    pub source_prompt_type: &'a str,
    pub first_frame_prompts: &'a [VideoFramePrompt],
}

pub(in crate::video) fn first_frame_prompt_drafts(
    input: FirstFramePromptDraftsInput<'_>,
) -> Vec<NewChapterVideoPrompt> {
    let FirstFramePromptDraftsInput {
        common,
        source_prompt_type,
        first_frame_prompts,
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
    first_frame_prompts
        .iter()
        .map(|frame| {
            let metadata = with_prompt_output_language_metadata(
                json!({
                    "contract_version": FIRST_FRAME_CHAIN_CONTRACT_VERSION,
                    "first_frame": frame.clone(),
                    "reference_slots": super::super::reference_slots_for_frame(
                        frame.reference_image_ids.len(),
                        None,
                    ),
                    "blocking_state": frame.blocking_state.clone(),
                    "lighting_state": frame.lighting_state.clone(),
                    "available_assets": available_assets.clone(),
                    "generation_asset_fingerprint": generation_asset_fingerprint.clone(),
                    "video_execution_identity": execution_identity.clone(),
                    "source_prompt_type": source_prompt_type,
                    "source": source_metadata.clone()
                }),
                prompt_output_language,
            );
            new_prompt_draft(NewChapterVideoPromptInput {
                project_id,
                chapter_id,
                workflow_mode: PromptWorkflowMode::FirstFrameChain,
                style: Some(style),
                prompt_type: "first_frame_prompt",
                shot_number: frame.frame_number,
                title: Some(&frame.title),
                scene_summary: Some(&frame.scene_summary),
                visual_prompt: &frame.frame_prompt,
                motion_prompt: None,
                negative_prompt: Some(&frame.negative_prompt),
                duration_secs: frame.duration_secs,
                aspect_ratio: Some(&frame.aspect_ratio),
                reference_image_ids_json: Some(json!(frame.reference_image_ids)),
                metadata_json: Some(metadata),
            })
        })
        .collect()
}

pub(in crate::video) struct StoryboardPromptDraftsInput<'a> {
    pub common: VideoPromptDraftCommonInput<'a>,
    pub source_prompt_type: &'a str,
    pub video_units: &'a [VideoUnitPlan],
    pub storyboard_prompts: &'a [VideoFramePrompt],
}

pub(in crate::video) fn storyboard_prompt_drafts(
    input: StoryboardPromptDraftsInput<'_>,
) -> Vec<NewChapterVideoPrompt> {
    let StoryboardPromptDraftsInput {
        common,
        source_prompt_type,
        video_units,
        storyboard_prompts,
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
    storyboard_prompts
        .iter()
        .map(|frame| {
            let video_unit = video_units
                .iter()
                .find(|unit| unit.unit_number == frame.frame_number);
            let metadata = with_prompt_output_language_metadata(
                json!({
                    "contract_version": STORYBOARD_PROMPT_CONTRACT_VERSION,
                    "storyboard_frame": frame.clone(),
                    "video_unit": video_unit,
                    "reference_slots": super::super::reference_slots_for_frame(
                        frame.reference_image_ids.len(),
                        None,
                    ),
                    "blocking_state": frame.blocking_state.clone(),
                    "lighting_state": frame.lighting_state.clone(),
                    "available_assets": available_assets.clone(),
                    "generation_asset_fingerprint": generation_asset_fingerprint.clone(),
                    "video_execution_identity": execution_identity.clone(),
                    "source_prompt_type": source_prompt_type,
                    "source": source_metadata.clone()
                }),
                prompt_output_language,
            );
            new_prompt_draft(NewChapterVideoPromptInput {
                project_id,
                chapter_id,
                workflow_mode: PromptWorkflowMode::Storyboard,
                style: Some(style),
                prompt_type: "storyboard_prompt",
                shot_number: frame.frame_number,
                title: Some(&frame.title),
                scene_summary: Some(&frame.scene_summary),
                visual_prompt: &frame.frame_prompt,
                motion_prompt: None,
                negative_prompt: Some(&frame.negative_prompt),
                duration_secs: frame.duration_secs,
                aspect_ratio: Some(&frame.aspect_ratio),
                reference_image_ids_json: Some(json!(frame.reference_image_ids)),
                metadata_json: Some(metadata),
            })
        })
        .collect()
}
