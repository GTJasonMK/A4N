use std::collections::HashMap;

use a4n_core::error::AppError;
use a4n_core::models::chapter::chapter_video_prompt;

use crate::video::{VideoPromptChunk, VideoPromptProgress};

mod contracts;
mod finalization;
mod inputs;
mod preparation;
mod reuse_preparation;
mod service_facade;
mod unit_execution;
mod unit_workflow;

use finalization::{VideoPromptFinalizationInput, finalize_video_prompt_chain};
use inputs::ChapterVideoPromptChainInput;
use preparation::{
    PreparedVideoChainInputs, VideoChainPreparationInput, prepare_video_chain_inputs,
};
use reuse_preparation::{
    PreparedVideoPromptReuse, VideoPromptReusePreparationInput, prepare_video_prompt_reuse,
};
use unit_workflow::{
    GeneratedVideoPromptUnits, VideoPromptUnitWorkflowInput, generate_video_prompt_units,
};

async fn generate_chapter_video_prompt_chain(
    input: ChapterVideoPromptChainInput<'_, '_>,
    mut chunks: Option<VideoPromptChunk<'_>>,
    mut progress: Option<VideoPromptProgress<'_>>,
) -> Result<Vec<chapter_video_prompt::Model>, AppError> {
    let ChapterVideoPromptChainInput {
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
    let is_chat_video = spec.is_chat_video();
    let workflow_run_id = options.workflow_run_id;
    let PreparedVideoChainInputs {
        source_metadata,
        regenerate_existing,
        prompt_batch_size,
        target_shot_number,
        segment_recipe,
        style,
        prompt_output_language,
        duration_secs,
        aspect_ratio,
        reference_ids,
        available_assets,
        generation_asset_fingerprint,
        execution_identity,
        vars,
        allowed_reference_ids,
        all_video_units,
        frame_json,
        frame_by_number,
        video_segment_plan,
        video_segment_json,
        video_unit_by_number,
        segment_storage_prompt_type,
    } = prepare_video_chain_inputs(
        VideoChainPreparationInput {
            service,
            spec,
            project_id,
            chapter_id,
            source: &source,
            variables,
            options,
        },
        &mut progress,
    )
    .await?;
    let segment_by_number = video_segment_plan
        .iter()
        .map(|segment| (segment.segment_number, segment))
        .collect::<HashMap<_, _>>();
    if let Some(target) = target_shot_number
        && !segment_by_number.contains_key(&target)
    {
        return Err(AppError::InvalidArgument(format!(
            "片段规划中不存在目标片段 {target}"
        )));
    }

    let PreparedVideoPromptReuse {
        generation_limit,
        reusable_video_prompts,
        existing_generated_video_prompts,
        matching_generated_video_shots,
    } = prepare_video_prompt_reuse(VideoPromptReusePreparationInput {
        service,
        spec,
        project_id,
        chapter_id,
        source_metadata: &source_metadata,
        style: &style,
        duration_secs,
        aspect_ratio: &aspect_ratio,
        reference_ids: &reference_ids,
        generation_asset_fingerprint: &generation_asset_fingerprint,
        execution_identity: &execution_identity,
        segment_storage_prompt_type,
        prompt_output_language,
        video_segment_plan: &video_segment_plan,
        regenerate_existing,
        prompt_batch_size,
    })
    .await?;
    let GeneratedVideoPromptUnits {
        video_prompts,
        context_video_prompts,
        preserved_generated_reused_shots,
        generated_video_prompt_traces,
        generated_this_run,
    } = generate_video_prompt_units(
        VideoPromptUnitWorkflowInput {
            service,
            spec,
            project_id,
            chapter_id,
            source: &source,
            prompt_renderer,
            vars: &vars,
            workflow_run_id,
            cancel_token,
            video_segment_plan: &video_segment_plan,
            segment_by_number: &segment_by_number,
            video_segment_json: &video_segment_json,
            frame_json: &frame_json,
            available_assets: &available_assets,
            style: &style,
            prompt_output_language,
            duration_secs,
            aspect_ratio: &aspect_ratio,
            reference_ids: &reference_ids,
            allowed_reference_ids: &allowed_reference_ids,
            is_chat_video,
            generation_limit,
            reusable_video_prompts: &reusable_video_prompts,
            existing_generated_video_prompts: &existing_generated_video_prompts,
            matching_generated_video_shots: &matching_generated_video_shots,
            regenerate_existing,
            target_shot_number,
        },
        &mut progress,
        &mut chunks,
    )
    .await?;

    finalize_video_prompt_chain(
        VideoPromptFinalizationInput {
            service,
            spec,
            project_id,
            chapter_id,
            style: &style,
            prompt_output_language,
            segment_recipe: &segment_recipe,
            source_metadata: &source_metadata,
            segment_by_number: &segment_by_number,
            video_unit_by_number: &video_unit_by_number,
            frame_by_number: &frame_by_number,
            available_assets: &available_assets,
            generation_reference_ids: &reference_ids,
            generation_asset_fingerprint: &generation_asset_fingerprint,
            execution_identity: &execution_identity,
            video_prompts: &video_prompts,
            context_video_prompts: &context_video_prompts,
            preserved_generated_reused_shots: &preserved_generated_reused_shots,
            generated_video_prompt_traces: &generated_video_prompt_traces,
            generated_this_run,
            all_video_units: &all_video_units,
            video_segment_plan: &video_segment_plan,
            regenerate_existing,
        },
        &mut progress,
    )
    .await
}
