use a4n_core::error::AppError;

use crate::video::prompt_generation::frame_prompts::{
    FrameImageLookupContext, FrameImageLookupRowsQuery, boundary_frame_image_ids_by_number,
    boundary_frame_prompt_rows_prefix, boundary_frame_rows_for_image_lookup,
    ensure_no_orphan_boundary_frame_images, frame_prompts_from_rows,
    reusable_frame_prefix_len_with_previous_images,
    sync_reusable_boundary_frame_previous_image_refs,
};

use super::contracts::{BoundaryFrameReuseInput, PreparedBoundaryFrameReuse};

pub(in crate::video::prompt_generation::frame_prompts::frame_chain) async fn prepare_boundary_frame_reuse(
    input: BoundaryFrameReuseInput<'_>,
) -> Result<PreparedBoundaryFrameReuse, AppError> {
    ensure_no_orphan_boundary_frame_images(
        input.service,
        input.project_id,
        input.chapter_id,
        input.spec.workflow_mode.as_str(),
        input.spec.frame_prompt_type,
    )
    .await?;
    let frame_image_lookup_rows = if input.regenerate_existing {
        Vec::new()
    } else {
        boundary_frame_rows_for_image_lookup(FrameImageLookupRowsQuery {
            service: input.service,
            project_id: input.project_id,
            chapter_id: input.chapter_id,
            context: FrameImageLookupContext {
                workflow_mode: input.spec.workflow_mode,
                prompt_type: input.spec.frame_prompt_type,
                source_metadata: input.source_metadata,
                style: input.style,
                aspect_ratio: input.aspect_ratio,
                video_execution_identity: input.execution_identity,
                source_prompt_type: input.frame_source_prompt_type,
                prompt_output_language: input.prompt_output_language,
            },
        })
        .await?
    };
    let boundary_frame_image_id_by_number = boundary_frame_image_ids_by_number(
        input.service,
        input.project_id,
        input.chapter_id,
        &frame_image_lookup_rows,
    )
    .await?;
    let reusable_rows = sync_reusable_boundary_frame_previous_image_refs(
        input.service,
        input.reusable_rows,
        &boundary_frame_image_id_by_number,
    )
    .await?;
    let mut allowed_reference_ids = input.allowed_reference_ids.clone();
    allowed_reference_ids.extend(boundary_frame_image_id_by_number.values().copied());
    let mut prefix_rows = boundary_frame_prompt_rows_prefix(
        reusable_rows,
        input.segment_plan,
        &allowed_reference_ids,
        &boundary_frame_image_id_by_number,
    )?;
    let mut frame_prompts = frame_prompts_from_rows(&prefix_rows)?;
    let reusable_prefix_len = reusable_frame_prefix_len_with_previous_images(
        &frame_prompts,
        &boundary_frame_image_id_by_number,
    );
    if reusable_prefix_len < frame_prompts.len() {
        tracing::info!(
            target: "a4n::video_prompt",
            project_id = input.project_id,
            chapter_id = input.chapter_id,
            reusable_prefix_len,
            original_prefix_len = frame_prompts.len(),
            "边界帧提示词缺少已生成上一帧普通参考图，将从该帧开始重新生成"
        );
        prefix_rows.truncate(reusable_prefix_len);
        frame_prompts.truncate(reusable_prefix_len);
    }

    Ok(PreparedBoundaryFrameReuse {
        boundary_frame_image_id_by_number,
        allowed_reference_ids,
        prefix_rows,
        frame_prompts,
    })
}
