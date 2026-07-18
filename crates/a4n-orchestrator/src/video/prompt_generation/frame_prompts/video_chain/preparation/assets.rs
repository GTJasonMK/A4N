use std::collections::HashMap;

use a4n_core::error::AppError;
use serde_json::json;

use super::super::super::{
    PromptWorkflowMode, available_video_image_ids, ensure_video_reference_ids_known,
    report_video_prompt_progress, video_generation_asset_fingerprint,
    video_prompt_unit_segments_with_reference_ids, video_prompt_workflow_reference_ids,
};
use super::contracts::{FinalizedVideoChainAssets, VideoChainAssetFinalizationInput};

pub(super) async fn finalize_video_chain_assets(
    input: VideoChainAssetFinalizationInput<'_, '_, '_>,
) -> Result<FinalizedVideoChainAssets, AppError> {
    let VideoChainAssetFinalizationInput {
        service,
        spec,
        project_id,
        chapter_id,
        source,
        reference_image_ids,
        video_library_image_ids,
        mut vars,
        selected_frames,
        progress,
    } = input;
    let frame_json = json!({ "frames": selected_frames.frame_prompts });
    let frame_by_number = selected_frames
        .frame_prompts
        .iter()
        .map(|frame| (frame.frame_number, frame.clone()))
        .collect::<HashMap<_, _>>();
    let video_segment_plan = video_prompt_unit_segments_with_reference_ids(
        spec.workflow_mode,
        &selected_frames.prompt_video_units,
        &frame_by_number,
        &selected_frames.frame_image_ids_by_number,
        &reference_image_ids,
    )?;
    let reference_image_ids = video_prompt_workflow_reference_ids(&video_segment_plan);
    let reference_ids = json!(reference_image_ids.clone());
    let available_assets = service
        .video_library_image_context(
            project_id,
            chapter_id,
            source,
            &reference_image_ids,
            &video_library_image_ids,
        )
        .await?;
    let generation_asset_fingerprint = video_generation_asset_fingerprint(
        &reference_image_ids,
        &video_library_image_ids,
        &available_assets,
    );
    vars.insert("reference_image_ids".into(), reference_ids.clone());
    vars.insert("available_video_assets".into(), available_assets.clone());
    let allowed_reference_ids = available_video_image_ids(&available_assets, &reference_image_ids)?;
    if matches!(
        spec.workflow_mode,
        PromptWorkflowMode::BoundaryFrames | PromptWorkflowMode::ChatVideo
    ) && video_segment_plan.len() < selected_frames.all_video_units.len()
    {
        report_video_prompt_progress(
            progress,
            45,
            100,
            &format!(
                "已找到 {} 条可用边界帧，可生成 {}/{} 个视频单元提示词",
                selected_frames.frame_prompts.len(),
                video_segment_plan.len(),
                selected_frames.all_video_units.len()
            ),
        )?;
    }
    for segment in &video_segment_plan {
        ensure_video_reference_ids_known(
            &segment.reference_image_ids,
            &allowed_reference_ids,
            &format!("{} #{}", spec.video_prompt_label, segment.segment_number),
        )?;
    }
    let video_segment_json = json!({ "segments": &video_segment_plan });
    let video_unit_by_number = selected_frames
        .prompt_video_units
        .iter()
        .map(|unit| (unit.unit_number, unit.clone()))
        .collect::<HashMap<_, _>>();

    Ok(FinalizedVideoChainAssets {
        reference_ids,
        available_assets,
        generation_asset_fingerprint,
        vars,
        allowed_reference_ids,
        all_video_units: selected_frames.all_video_units,
        frame_json,
        frame_by_number,
        video_segment_plan,
        video_segment_json,
        video_unit_by_number,
    })
}
