use std::collections::{HashMap, HashSet};

use serde_json::Value;

use super::super::{
    AppError, ChatVideoPromptDraftsInput, FirstFrameVideoPromptDraftsInput,
    FramePromptGenerationSpec, GeneratedVideoPromptShot, NewChapterVideoPrompt,
    PromptOutputLanguage, PromptWorkflowMode, StoryboardVideoPromptDraftsInput, VideoFramePrompt,
    VideoPromptDraftCommonInput, VideoPromptDraftsInput, VideoSegmentPlan, VideoUnitPlan,
    api_generated_video_prompts, chat_generated_video_prompts, chat_video_prompt_drafts,
    first_frame_video_prompt_drafts, storyboard_video_prompt_drafts, video_prompt_drafts,
};

pub(super) struct VideoPromptDraftsForSpecInput<'a> {
    pub spec: &'a FramePromptGenerationSpec,
    pub project_id: i64,
    pub chapter_id: i64,
    pub style: &'a str,
    pub prompt_output_language: PromptOutputLanguage,
    pub source_metadata: &'a Value,
    pub segment_by_number: &'a HashMap<i32, &'a VideoSegmentPlan>,
    pub video_unit_by_number: &'a HashMap<i32, VideoUnitPlan>,
    pub frame_by_number: &'a HashMap<i32, VideoFramePrompt>,
    pub available_assets: &'a Value,
    pub generation_reference_ids: &'a Value,
    pub generation_asset_fingerprint: &'a Value,
    pub execution_identity: &'a Value,
    pub video_prompts: &'a [GeneratedVideoPromptShot],
    pub preserved_generated_reused_shots: &'a HashSet<i32>,
}

pub(super) fn video_prompt_drafts_for_spec(
    input: VideoPromptDraftsForSpecInput<'_>,
) -> Result<Vec<NewChapterVideoPrompt>, AppError> {
    let VideoPromptDraftsForSpecInput {
        spec,
        project_id,
        chapter_id,
        style,
        prompt_output_language,
        source_metadata,
        segment_by_number,
        video_unit_by_number,
        frame_by_number,
        available_assets,
        generation_reference_ids,
        generation_asset_fingerprint,
        execution_identity,
        video_prompts,
        preserved_generated_reused_shots,
    } = input;
    let writable_video_prompts =
        writable_generated_video_prompts(video_prompts, preserved_generated_reused_shots);
    let common = VideoPromptDraftCommonInput {
        project_id,
        chapter_id,
        style,
        prompt_output_language,
        source_metadata,
        available_assets,
        generation_asset_fingerprint,
        execution_identity,
    };
    match spec.workflow_mode {
        PromptWorkflowMode::SharedSegmentPlan => Err(AppError::InvalidArgument(
            "共享片段规划不能写入卡片最终提示词".to_string(),
        )),
        PromptWorkflowMode::BoundaryFrames => video_prompt_drafts(VideoPromptDraftsInput {
            common,
            segment_by_number,
            video_unit_by_number,
            frame_by_number,
            video_prompts: &api_generated_video_prompts(
                &writable_video_prompts,
                spec.video_prompt_label,
            )?,
        }),
        PromptWorkflowMode::FirstFrameChain => {
            first_frame_video_prompt_drafts(FirstFrameVideoPromptDraftsInput {
                common,
                source_prompt_type: spec.frame_source_prompt_type(),
                segment_by_number,
                video_unit_by_number,
                first_frame_by_number: frame_by_number,
                video_prompts: &api_generated_video_prompts(
                    &writable_video_prompts,
                    spec.video_prompt_label,
                )?,
            })
        }
        PromptWorkflowMode::ChatVideo => {
            let writable_prompts =
                chat_generated_video_prompts(&writable_video_prompts, spec.video_prompt_label)?;
            chat_video_prompt_drafts(ChatVideoPromptDraftsInput {
                common,
                source_prompt_type: spec.segment_storage_prompt_type(),
                segment_by_number,
                video_unit_by_number,
                generation_reference_ids,
                video_prompts: &writable_prompts,
            })
        }
        PromptWorkflowMode::Storyboard => {
            storyboard_video_prompt_drafts(StoryboardVideoPromptDraftsInput {
                common,
                segment_by_number,
                video_unit_by_number,
                storyboard_by_number: frame_by_number,
                video_prompts: &api_generated_video_prompts(
                    &writable_video_prompts,
                    spec.video_prompt_label,
                )?,
            })
        }
    }
}

fn writable_generated_video_prompts(
    prompts: &[GeneratedVideoPromptShot],
    preserved_generated_reused_shots: &HashSet<i32>,
) -> Vec<GeneratedVideoPromptShot> {
    prompts
        .iter()
        .filter(|prompt| !preserved_generated_reused_shots.contains(&prompt.shot_number()))
        .cloned()
        .collect()
}
