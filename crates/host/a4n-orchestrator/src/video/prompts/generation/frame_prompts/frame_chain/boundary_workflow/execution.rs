use a4n_error::AppError;

use crate::video::prompts::generation::frame_prompts::{
    NewChapterVideoPrompt, report_video_prompt_item_committed,
};

use super::super::boundary_frames::{
    BoundaryFrameCheckpointInput, BoundaryFrameFinalizationInput, BoundaryFramePersistenceInput,
    BoundaryFrameRequestInput, BoundaryFrameReuseInput, finalize_boundary_frame_batch,
    persist_boundary_frame_checkpoint, prepare_boundary_frame_request,
    render_boundary_frame_checkpoint,
};
use super::contracts::{
    BoundaryFrameWorkflowInput, BoundaryIterationInput, BoundaryWorkflowPreparationInput,
    PendingBoundaryWorkflow, PreparedBoundaryWorkflow,
};
use super::iteration::prepare_boundary_iteration;
use super::preparation::prepare_boundary_workflow;

pub(in crate::video::prompts::generation::frame_prompts::frame_chain) async fn generate_boundary_frame_workflow(
    input: BoundaryFrameWorkflowInput<'_, '_, '_, '_>,
) -> Result<Vec<NewChapterVideoPrompt>, AppError> {
    let BoundaryFrameWorkflowInput {
        service,
        spec,
        project_id,
        chapter_id,
        source,
        source_metadata,
        source_spans,
        variables,
        project_context,
        available_assets,
        reference_image_ids,
        segment_json,
        segment_plan,
        style,
        prompt_output_language,
        aspect_ratio,
        frame_source_prompt_type,
        workflow_run_id,
        expected_frame_count,
        prompt_batch_size,
        regenerate_existing,
        reusable_rows,
        allowed_reference_ids,
        frame_draft_common,
        segment_draft,
        prompt_renderer,
        cancel_token,
        progress,
        chunks,
    } = input;
    let prepared = prepare_boundary_workflow(BoundaryWorkflowPreparationInput {
        reuse: BoundaryFrameReuseInput {
            service,
            spec,
            project_id,
            chapter_id,
            source_metadata,
            style,
            aspect_ratio,
            execution_identity: frame_draft_common.execution_identity,
            frame_source_prompt_type,
            prompt_output_language,
            segment_plan,
            allowed_reference_ids,
            reusable_rows,
            regenerate_existing,
        },
        expected_frame_count,
        reused_frame_label: spec.reused_frame_label,
        progress,
    })
    .await?;
    let PendingBoundaryWorkflow {
        boundary_frame_image_id_by_number,
        allowed_reference_ids: mut allowed_boundary_frame_reference_ids,
        mut frame_prompts,
        mut kept_prompt_ids,
    } = match prepared {
        PreparedBoundaryWorkflow::Complete(drafts) => return Ok(drafts),
        PreparedBoundaryWorkflow::Pending(pending) => pending,
    };

    let mut generated_this_run = 0usize;
    while frame_prompts.len() < expected_frame_count && generated_this_run < prompt_batch_size {
        let iteration = prepare_boundary_iteration(BoundaryIterationInput {
            service,
            spec,
            project_id,
            chapter_id,
            expected_frame_count,
            prompt_batch_size,
            generated_this_run,
            regenerate_existing,
            current_frame_count: frame_prompts.len(),
            next_frame_number: None,
            kept_prompt_ids: &kept_prompt_ids,
            progress,
        })
        .await?;
        let frame_number = iteration.frame_number;
        let generation_message = iteration.generation_message;
        let frame_progress_span = iteration.progress_span;
        let prepared_request = prepare_boundary_frame_request(BoundaryFrameRequestInput {
            prompt_renderer,
            spec,
            project_id,
            chapter_id,
            frame_number,
            source,
            source_spans,
            variables,
            project_context,
            available_assets,
            reference_image_ids,
            segment_json,
            segment_plan,
            frame_prompts: &frame_prompts,
            boundary_frame_image_id_by_number: &boundary_frame_image_id_by_number,
            style,
            prompt_output_language,
            aspect_ratio,
        })
        .await?;
        let (frame_trace, frame_prompt) =
            render_boundary_frame_checkpoint(BoundaryFrameCheckpointInput {
                service,
                spec,
                project_id,
                frame_number,
                workflow_run_id,
                prompt_renderer,
                variables,
                cancel_token,
                progress_span: frame_progress_span,
                generation_message: &generation_message,
                prepared_request,
                segment_plan,
                aspect_ratio,
                style,
                prompt_output_language,
                progress,
                chunks,
            })
            .await?;
        kept_prompt_ids = persist_boundary_frame_checkpoint(BoundaryFramePersistenceInput {
            service,
            spec,
            chapter_id,
            frame_number,
            workflow_run_id,
            frame_draft_common,
            frame_source_prompt_type,
            segment_draft,
            frame_trace,
            frame_prompt,
            frame_prompts: &mut frame_prompts,
            allowed_reference_ids: &mut allowed_boundary_frame_reference_ids,
        })
        .await?;
        generated_this_run += 1;
        report_video_prompt_item_committed(
            progress,
            frame_progress_span.end(),
            100,
            spec.frame_prompt_type,
            frame_number,
            frame_prompts.len(),
            expected_frame_count,
            &format!(
                "边界帧提示词第 {}/{} 张已保存，本批已生成 {} 条",
                frame_prompts.len(),
                expected_frame_count,
                generated_this_run
            ),
        )?;
    }

    finalize_boundary_frame_batch(BoundaryFrameFinalizationInput {
        spec,
        expected_frame_count,
        generated_this_run,
        frame_draft_common,
        frame_source_prompt_type,
        frame_prompts: &frame_prompts,
        segment_plan,
        allowed_reference_ids: &allowed_boundary_frame_reference_ids,
        boundary_frame_image_id_by_number: &boundary_frame_image_id_by_number,
        progress,
    })
}
