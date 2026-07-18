use super::super::persistence::{ensure_canvas_image_media_asset, ensure_canvas_video_media_asset};
use crate::AppState;

pub(in crate::actions::canvas_workflow) async fn sync_completed_canvas_image_asset(
    state: &AppState,
    project_id: i64,
    image_id: i64,
    generation_spec_id: Option<i64>,
    origin: &str,
) -> Result<Option<i64>, String> {
    let Some(generation_spec_id) = generation_spec_id else {
        return Ok(None);
    };
    let image = state
        .orchestrator
        .generated_image_by_id(image_id)
        .await
        .map_err(|err| err.to_string())?
        .ok_or_else(|| format!("画布图片产物不存在: {image_id}"))?;
    let asset = ensure_canvas_image_media_asset(
        state,
        project_id,
        &image,
        Some(generation_spec_id),
        origin,
    )
    .await?;
    Ok(Some(asset.id))
}

pub(in crate::actions::canvas_workflow) async fn sync_completed_canvas_video_asset(
    state: &AppState,
    project_id: i64,
    video_id: i64,
    generation_spec_id: Option<i64>,
    origin: &str,
) -> Result<Option<i64>, String> {
    let Some(generation_spec_id) = generation_spec_id else {
        return Ok(None);
    };
    let video = state
        .orchestrator
        .generated_video_by_id(video_id)
        .await
        .map_err(|err| err.to_string())?
        .ok_or_else(|| format!("画布视频产物不存在: {video_id}"))?;
    let asset = ensure_canvas_video_media_asset(
        state,
        project_id,
        &video,
        Some(generation_spec_id),
        origin,
    )
    .await?;
    Ok(Some(asset.id))
}
