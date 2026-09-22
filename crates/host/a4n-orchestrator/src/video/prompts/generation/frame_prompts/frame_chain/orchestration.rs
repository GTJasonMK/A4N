use a4n_db_models::models::chapter::chapter_video_prompt;
use a4n_error::AppError;
use serde_json::json;

use crate::video::prompts::generation::frame_prompts::{
    PreparedVideoPromptInputs, load_shared_segment_plan_for_execution, report_video_prompt_progress,
};
use crate::video::{VideoPromptChunk, VideoPromptProgress};

use super::contracts::FramePromptChainInput;
use super::finalization::{FrameChainFinalizationInput, finalize_frame_chain};
use super::frame_preparation::{
    FrameWorkflowPreparationInput, PreparedFrameWorkflow, prepare_frame_workflow,
};
use super::layout_dispatch::{FrameLayoutDispatchInput, generate_frame_layout};

pub(in crate::video::prompts::generation::frame_prompts) async fn generate_chapter_frame_prompt_chain(
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
        segment_plan_prompt_id,
        target_shot_number,
        style,
        segment_recipe,
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
    let segment_plan_prompt_id = segment_plan_prompt_id.ok_or_else(|| {
        AppError::InvalidArgument("帧提示词生成缺少 segment_plan_prompt_id".to_string())
    })?;
    let segment_recipe = segment_recipe
        .ok_or_else(|| AppError::InvalidArgument("帧提示词生成缺少 segment_recipe".to_string()))?;
    let target_shot_number = target_shot_number
        .ok_or_else(|| AppError::InvalidArgument("帧提示词生成缺少目标片段".to_string()))?;
    report_video_prompt_progress(&mut progress, 20, 100, "已加载当前共享片段规划")?;
    let loaded_plan = load_shared_segment_plan_for_execution(
        service,
        project_id,
        chapter_id,
        segment_plan_prompt_id,
    )
    .await?;
    let crate::video::prompts::generation::frame_prompts::LoadedSharedSegmentPlan {
        segment_plan: loaded_segment_plan,
        execution_identity,
    } = loaded_plan;
    let segment = loaded_segment_plan
        .into_iter()
        .find(|segment| segment.segment_number == target_shot_number)
        .ok_or_else(|| {
            AppError::InvalidArgument(format!("共享片段规划中不存在目标片段 {target_shot_number}"))
        })?;
    let target = (
        segment.source_shot_id.clone(),
        segment.segment_index_in_shot,
    );
    let segment_plan = vec![segment];
    let segment_json = json!({ "segments": &segment_plan });
    let segment_draft = None;
    let execution_identity = super::super::video_segment_frame_execution_identity(
        execution_identity,
        Some(segment_plan_prompt_id),
        Some(&segment_recipe),
        (&target.0, target.1),
    )?;
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
        target_shot_number: Some(target_shot_number),
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
        target_shot_number: Some(target_shot_number),
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
        regenerate_existing,
        segment_draft,
        frame_drafts,
        progress: &mut progress,
    })
    .await
}
