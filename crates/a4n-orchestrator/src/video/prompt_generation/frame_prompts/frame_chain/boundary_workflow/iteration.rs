use a4n_core::error::AppError;

use crate::video::prompt_generation::frame_prompts::{
    ProgressSpan, ensure_boundary_frame_checkpoint_can_sync, report_video_prompt_progress,
};

use super::contracts::{BoundaryIterationInput, PreparedBoundaryIteration};

pub(super) async fn prepare_boundary_iteration(
    input: BoundaryIterationInput<'_, '_>,
) -> Result<PreparedBoundaryIteration, AppError> {
    ensure_boundary_frame_checkpoint_can_sync(
        input.service,
        input.project_id,
        input.chapter_id,
        input.spec.workflow_mode,
        input.spec.frame_prompt_type,
        input.kept_prompt_ids,
    )
    .await?;
    let next_frame_count = input
        .current_frame_count
        .checked_add(1)
        .ok_or_else(|| AppError::InvalidArgument("边界帧数量超过 usize 上限".to_string()))?;
    let frame_number = i32::try_from(next_frame_count).map_err(|_| {
        AppError::InvalidArgument(format!("边界帧编号超过 i32 上限：{next_frame_count}"))
    })?;
    let generated_this_run = input.generated_this_run.checked_add(1).ok_or_else(|| {
        AppError::InvalidArgument("本批边界帧生成数量超过 usize 上限".to_string())
    })?;
    let message = if input.regenerate_existing {
        input.spec.frame_regenerate_progress
    } else {
        input.spec.frame_generate_progress
    };
    let generation_message = format!(
        "{message}：正在生成第 {}/{} 张边界帧，本批第 {}/{} 条",
        frame_number, input.expected_frame_count, generated_this_run, input.prompt_batch_size
    );
    let progress_span =
        ProgressSpan::new(60, 90).split(input.current_frame_count, input.expected_frame_count);
    report_video_prompt_progress(
        input.progress,
        progress_span.start(),
        100,
        &generation_message,
    )?;
    Ok(PreparedBoundaryIteration {
        frame_number,
        generation_message,
        progress_span,
    })
}
