use super::super::{WORKFLOW_STATUS_COMPLETED, types::CanvasVideoMediaAssetEnsureInput};
use super::{
    CanvasMediaAssetUpsertInput, binding::ensure_canvas_media_binding,
    context::canvas_media_source_context_for_update,
    lookup::canvas_generated_video_for_media_ensure,
    provenance::upsert_canvas_video_media_provenance, upsert_canvas_media_asset_record,
};
use crate::canvas::CanvasDbError;
use a4n_core::models::media::media_asset;
use sea_orm::{ConnectionTrait, TransactionTrait};

pub async fn ensure_canvas_video_media_asset_record<C>(
    db: &C,
    input: CanvasVideoMediaAssetEnsureInput,
) -> Result<media_asset::Model, CanvasDbError>
where
    C: ConnectionTrait + TransactionTrait,
{
    let tx = db.begin().await?;
    let asset = ensure_canvas_video_media_asset_record_in(&tx, input).await?;
    tx.commit().await?;
    Ok(asset)
}

pub(in crate::canvas::artifacts) async fn ensure_canvas_video_media_asset_record_in<C>(
    db: &C,
    input: CanvasVideoMediaAssetEnsureInput,
) -> Result<media_asset::Model, CanvasDbError>
where
    C: ConnectionTrait,
{
    let expected_chapter_id = input.video.chapter_id;
    let source_context =
        canvas_media_source_context_for_update(db, input.project_id, expected_chapter_id).await?;
    let video =
        canvas_generated_video_for_media_ensure(db, input.project_id, input.video.id).await?;
    if video.chapter_id != expected_chapter_id {
        return Err(CanvasDbError::InvalidArgument(format!(
            "视频 {} 的章节归属在媒体登记期间发生变化，请重试",
            video.id
        )));
    }
    if video.status != WORKFLOW_STATUS_COMPLETED {
        return Err(CanvasDbError::InvalidArgument(format!(
            "视频尚未完成，不能登记为媒体资产: {}",
            video.id
        )));
    }
    let file_path = video
        .file_path
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            CanvasDbError::InvalidArgument(format!(
                "视频缺少文件路径，不能登记为媒体资产: {}",
                video.id
            ))
        })?
        .to_string();
    let asset = upsert_canvas_media_asset_record(
        db,
        CanvasMediaAssetUpsertInput {
            project_id: input.project_id,
            media_kind: "video",
            origin: input.origin,
            file_path,
            thumbnail_path: video.thumbnail_path.clone(),
            width: Some(video.width),
            height: Some(video.height),
            duration_secs: Some(video.duration_secs),
            fps: Some(video.fps as f32),
            file_size: video.file_size,
        },
        &source_context,
    )
    .await?;

    upsert_canvas_video_media_provenance(db, &asset, &video, input.generation_spec_id).await?;
    ensure_canvas_media_binding(
        db,
        asset.id,
        "generated_video",
        video.id,
        "generated_result",
    )
    .await?;
    Ok(asset)
}
