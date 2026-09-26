use a4n_error::AppError;

use crate::video::library::available_video_image_ids;
use crate::video::prompts::context::{
    FramePromptContext, VideoContextManifestInput, build_single_frame_prompt_user,
    build_video_context_manifest,
};
use crate::video::prompts::generation::frame_prompts::{
    available_assets_with_previous_boundary_frame, backend_frame_reference_ids,
    boundary_frame_generation_context, frame_prompt_available_assets_context,
    previous_boundary_frame_asset_context, video_stage_fingerprint,
};

use super::contracts::{BoundaryFrameRequestInput, PreparedBoundaryFrameRequest};

pub(in crate::video::prompts::generation::frame_prompts::frame_chain) async fn prepare_boundary_frame_request(
    input: BoundaryFrameRequestInput<'_, '_>,
) -> Result<PreparedBoundaryFrameRequest, AppError> {
    let previous_frame_image_id = if input.frame_number > 1 {
        input
            .boundary_frame_image_id_by_number
            .get(&(input.frame_number - 1))
            .copied()
    } else {
        None
    };
    let frame_available_assets = available_assets_with_previous_boundary_frame(
        input.available_assets,
        previous_frame_image_id,
        input.frame_prompts.last(),
    );
    let prompt_available_assets = frame_prompt_available_assets_context(&frame_available_assets)?;
    let allowed_reference_ids =
        available_video_image_ids(&frame_available_assets, input.reference_image_ids)?;
    let reference_image_ids =
        backend_frame_reference_ids(input.reference_image_ids, previous_frame_image_id);
    let reference_ids = serde_json::json!(reference_image_ids.clone());
    let current_frame = boundary_frame_generation_context(
        input.segment_plan,
        input.frame_prompts,
        input.frame_number,
        previous_boundary_frame_asset_context(&frame_available_assets, previous_frame_image_id),
    )?;
    let frame_user = build_single_frame_prompt_user(
        FramePromptContext {
            prompt_renderer: input.prompt_renderer,
            project_id: input.project_id,
            chapter_summary: input.source.summary,
            chapter_analysis: input.source.analysis,
            segment_plan: input.segment_json,
            available_assets: &prompt_available_assets,
            style: input.style,
            prompt_output_language: input.prompt_output_language,
            aspect_ratio: input.aspect_ratio,
            reference_ids: &reference_ids,
        },
        &current_frame,
    )
    .await?;
    let numbers = current_frame["expected_segment_numbers"]
        .as_array()
        .ok_or_else(|| AppError::InvalidArgument("边界帧缺少来源片段编号".into()))?;
    let selected_spans =
        crate::video::prompts::generation::frame_prompts::checkpoint::source_spans_for_segments(
            input.source_spans,
            input.segment_plan.iter().filter(|segment| {
                numbers
                    .iter()
                    .any(|number| number.as_i64() == Some(i64::from(segment.segment_number)))
            }),
        )?;
    let source_text = a4n_domain::source_spans::source_span_context_excerpt(
        input.source.content,
        &selected_spans,
    )
    .map_err(AppError::InvalidArgument)?;
    let scoped_source = crate::video::ChapterVideoSourceContext {
        content: &source_text,
        summary: input.source.summary,
        analysis: input.source.analysis,
    };
    let context_manifest = build_video_context_manifest(VideoContextManifestInput {
        prompt_renderer: input.prompt_renderer,
        project_id: input.project_id,
        variables: input.variables,
        source: &scoped_source,
        prompt_names: &["chapter_video_segment_plan", input.spec.frame_prompt_name],
        previous_outputs: &[],
        project_context: input.project_context,
        dynamic_user_content: frame_user,
    })
    .await?;
    let step_key = format!(
        "{}:frame:{}",
        input.spec.frame_prompt_name, input.frame_number
    );
    let fingerprint_json = video_stage_fingerprint(
        input.spec.frame_prompt_name,
        &context_manifest,
        serde_json::json!({
            "project_id": input.project_id,
            "chapter_id": input.chapter_id,
            "source_span_ids": selected_spans.iter().map(|span| &span.id).collect::<Vec<_>>(),
            "frame_number": input.frame_number,
            "frame_prompt_type": input.spec.frame_prompt_type,
        }),
    );

    Ok(PreparedBoundaryFrameRequest {
        previous_frame_image_id,
        allowed_reference_ids,
        reference_image_ids,
        context_manifest,
        step_key,
        fingerprint_json,
    })
}
