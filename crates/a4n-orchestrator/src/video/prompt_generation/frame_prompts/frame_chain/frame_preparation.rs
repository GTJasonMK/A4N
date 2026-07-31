use std::collections::HashSet;

use a4n_core::error::AppError;
use serde_json::Value;

use crate::video::library::available_video_image_ids;
use crate::video::prompt_generation::frame_prompts::{
    FramePromptGenerationSpec, NewChapterVideoPrompt, OrchestratorContext, ReusableFrameRowsQuery,
    VideoPromptDraftCommonInput, VideoPromptProgress, expected_boundary_frame_count,
    expected_video_unit_count, report_video_prompt_progress, reusable_frame_rows_for_resume,
    video_unit_for_target_segment, video_units_for_segment_plan,
};
use crate::video::prompt_json::VideoSegmentPlan;
use a4n_core::prompt_language::PromptOutputLanguage;

use crate::video::FIRST_FRAME_CHAIN_INITIAL_FRAME_COUNT;
use crate::video::prompt_generation::frame_prompts::{FramePromptLayout, VideoUnitPlan};

pub(super) struct FrameWorkflowPreparationInput<'a, 'progress, 'borrow> {
    pub service: &'a OrchestratorContext,
    pub spec: &'a FramePromptGenerationSpec,
    pub project_id: i64,
    pub chapter_id: i64,
    pub source_metadata: &'a Value,
    pub style: &'a str,
    pub prompt_output_language: PromptOutputLanguage,
    pub duration_secs: i32,
    pub aspect_ratio: &'a str,
    pub reference_image_ids: &'a [i64],
    pub reference_ids: &'a Value,
    pub generation_asset_fingerprint: &'a Value,
    pub execution_identity: &'a Value,
    pub available_assets: &'a Value,
    pub segment_plan: &'a [VideoSegmentPlan],
    pub target_shot_number: Option<i32>,
    pub segment_draft: Option<&'a NewChapterVideoPrompt>,
    pub regenerate_existing: bool,
    pub progress: &'borrow mut Option<VideoPromptProgress<'progress>>,
}

pub(super) struct PreparedFrameWorkflow<'a> {
    pub frame_source_prompt_type: &'a str,
    pub allowed_reference_ids: HashSet<i64>,
    pub expected_frame_count: usize,
    pub video_units: Vec<VideoUnitPlan>,
    pub frame_draft_common: VideoPromptDraftCommonInput<'a>,
    pub reusable_frame_rows: Vec<a4n_core::models::chapter::chapter_video_prompt::Model>,
}

pub(super) async fn prepare_frame_workflow<'a, 'progress>(
    input: FrameWorkflowPreparationInput<'a, 'progress, '_>,
) -> Result<PreparedFrameWorkflow<'a>, AppError> {
    let frame_source_prompt_type = input.spec.frame_source_prompt_type();
    let allowed_reference_ids =
        available_video_image_ids(input.available_assets, input.reference_image_ids)?;
    if !input.regenerate_existing {
        let mut initial_drafts = Vec::new();
        if let Some(draft) = input.segment_draft {
            initial_drafts.push(draft.clone());
        }
        if !initial_drafts.is_empty() {
            input
                .service
                .sync_chapter_prompts(input.chapter_id, initial_drafts)
                .await?;
        }
    }

    let expected_frame_count = match (input.spec.layout, input.target_shot_number) {
        (FramePromptLayout::BoundaryFrames, Some(_)) => 2,
        (FramePromptLayout::InitialFirstFrame, Some(1) | None) => {
            FIRST_FRAME_CHAIN_INITIAL_FRAME_COUNT
        }
        (FramePromptLayout::InitialFirstFrame, Some(target)) => {
            return Err(AppError::InvalidArgument(format!(
                "首帧续接片段 {target} 不生成新的首帧提示词；请先采用上一片段视频作为续接来源"
            )));
        }
        (FramePromptLayout::PerSegmentStoryboard, Some(_)) => 1,
        (FramePromptLayout::BoundaryFrames, None) => {
            expected_boundary_frame_count(input.segment_plan)
        }
        (FramePromptLayout::PerSegmentStoryboard, None) => {
            expected_video_unit_count(input.segment_plan)
        }
    };
    let video_units = match input.target_shot_number {
        Some(target) => vec![video_unit_for_target_segment(
            input.segment_plan,
            input.duration_secs,
            target,
        )?],
        None => video_units_for_segment_plan(input.segment_plan, input.duration_secs)?,
    };
    let frame_draft_common = VideoPromptDraftCommonInput {
        project_id: input.project_id,
        chapter_id: input.chapter_id,
        style: input.style,
        prompt_output_language: input.prompt_output_language,
        source_metadata: input.source_metadata,
        available_assets: input.available_assets,
        generation_asset_fingerprint: input.generation_asset_fingerprint,
        execution_identity: input.execution_identity,
    };
    report_video_prompt_progress(input.progress, 60, 100, input.spec.frame_check_progress)?;
    let reusable_frame_rows = if input.regenerate_existing
        || matches!(input.spec.layout, FramePromptLayout::InitialFirstFrame)
    {
        Vec::new()
    } else {
        reusable_frame_rows_for_resume(ReusableFrameRowsQuery {
            service: input.service,
            project_id: input.project_id,
            chapter_id: input.chapter_id,
            workflow_mode: input.spec.workflow_mode,
            prompt_type: input.spec.frame_prompt_type,
            source_metadata: input.source_metadata,
            style: input.style,
            duration_secs: input.duration_secs,
            aspect_ratio: input.aspect_ratio,
            reference_ids: input.reference_ids,
            generation_asset_fingerprint: input.generation_asset_fingerprint,
            source_prompt_type: frame_source_prompt_type,
            video_execution_identity: input.execution_identity,
            prompt_output_language: input.prompt_output_language,
        })
        .await?
    };

    Ok(PreparedFrameWorkflow {
        frame_source_prompt_type,
        allowed_reference_ids,
        expected_frame_count,
        video_units,
        frame_draft_common,
        reusable_frame_rows,
    })
}
