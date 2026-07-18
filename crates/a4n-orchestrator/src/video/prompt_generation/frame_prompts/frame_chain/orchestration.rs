use a4n_core::error::AppError;
use a4n_core::models::chapter::chapter_video_prompt;

use crate::video::prompt_generation::frame_prompts::PreparedVideoPromptInputs;
use crate::video::{VideoPromptChunk, VideoPromptProgress};

use super::contracts::FramePromptChainInput;
use super::finalization::{FrameChainFinalizationInput, finalize_frame_chain};
use super::frame_preparation::{
    FrameWorkflowPreparationInput, PreparedFrameWorkflow, prepare_frame_workflow,
};
use super::layout_dispatch::{FrameLayoutDispatchInput, generate_frame_layout};
use super::planning_workflow::{
    PlanningWorkflowInput, PreparedPlanningWorkflow, prepare_frame_chain_planning,
};

pub(in crate::video::prompt_generation::frame_prompts) async fn generate_chapter_frame_prompt_chain(
    input: FramePromptChainInput<'_, '_>,
    mut chunks: Option<VideoPromptChunk<'_>>,
    mut progress: Option<VideoPromptProgress<'_>>,
) -> Result<Vec<chapter_video_prompt::Model>, AppError> {
    let FramePromptChainInput {
        service,
        spec,
        project_id,
        chapter_id,
        source,
        prompt_renderer,
        variables,
        options,
        cancel_token,
    } = input;
    let PreparedVideoPromptInputs {
        source_metadata,
        source_spans,
        regenerate_existing,
        prompt_batch_size,
        style,
        prompt_output_language,
        duration_secs,
        aspect_ratio,
        reference_image_ids,
        reference_ids,
        generation_asset_fingerprint,
        project_context,
        available_assets,
        vars,
        workflow_run_id,
        ..
    } = service
        .prepare_video_prompt_inputs(project_id, chapter_id, &source, variables, options, true)
        .await?;
    let PreparedPlanningWorkflow {
        segment_json,
        segment_plan,
        segment_draft,
        execution_identity,
    } = prepare_frame_chain_planning(PlanningWorkflowInput {
        service,
        spec,
        project_id,
        chapter_id,
        source,
        source_metadata: &source_metadata,
        source_spans: &source_spans,
        project_context: project_context.as_ref(),
        style: &style,
        prompt_output_language,
        duration_secs,
        aspect_ratio: &aspect_ratio,
        variables: &vars,
        regenerate_existing,
        workflow_run_id,
        prompt_renderer,
        cancel_token,
        progress: &mut progress,
        chunks: &mut chunks,
    })
    .await?;
    let PreparedFrameWorkflow {
        frame_source_prompt_type,
        allowed_reference_ids,
        expected_frame_count,
        video_units,
        frame_draft_common,
        reusable_frame_rows,
    } = prepare_frame_workflow(FrameWorkflowPreparationInput {
        service,
        spec,
        project_id,
        chapter_id,
        source_metadata: &source_metadata,
        style: &style,
        prompt_output_language,
        duration_secs,
        aspect_ratio: &aspect_ratio,
        reference_image_ids: &reference_image_ids,
        reference_ids: &reference_ids,
        generation_asset_fingerprint: &generation_asset_fingerprint,
        execution_identity: &execution_identity,
        available_assets: &available_assets,
        segment_plan: &segment_plan,
        segment_draft: segment_draft.as_ref(),
        regenerate_existing,
        progress: &mut progress,
    })
    .await?;
    let frame_drafts = generate_frame_layout(FrameLayoutDispatchInput {
        service,
        spec,
        project_id,
        chapter_id,
        source,
        source_metadata: &source_metadata,
        source_spans: &source_spans,
        variables: &vars,
        project_context: project_context.as_ref(),
        available_assets: &available_assets,
        reference_image_ids: &reference_image_ids,
        reference_ids: &reference_ids,
        allowed_reference_ids: &allowed_reference_ids,
        segment_json: &segment_json,
        segment_plan: &segment_plan,
        video_units: &video_units,
        style: &style,
        prompt_output_language,
        aspect_ratio: &aspect_ratio,
        frame_source_prompt_type,
        workflow_run_id,
        expected_frame_count,
        prompt_batch_size,
        regenerate_existing,
        reusable_frame_rows,
        frame_draft_common,
        segment_draft: segment_draft.as_ref(),
        prompt_renderer,
        cancel_token,
        progress: &mut progress,
        chunks: &mut chunks,
    })
    .await?;
    finalize_frame_chain(FrameChainFinalizationInput {
        service,
        spec,
        chapter_id,
        expected_frame_count,
        regenerate_existing,
        segment_draft,
        frame_drafts,
        progress: &mut progress,
    })
    .await
}
