use super::super::types::CanvasImageMediaAssetEnsureInput;
use super::{
    CanvasMediaAssetUpsertInput, binding::ensure_canvas_media_binding,
    context::canvas_media_source_context_for_update,
    lookup::canvas_generated_image_for_media_ensure,
    provenance::upsert_canvas_image_media_provenance, upsert_canvas_media_asset_record,
};
use crate::canvas::CanvasDbError;
use a4n_db_models::models::media::media_asset;
use sea_orm::{ConnectionTrait, TransactionTrait};

pub async fn ensure_canvas_image_media_asset_record<C>(
    db: &C,
    input: CanvasImageMediaAssetEnsureInput,
) -> Result<media_asset::Model, CanvasDbError>
where
    C: ConnectionTrait + TransactionTrait,
{
    let tx = db.begin().await?;
    let asset = ensure_canvas_image_media_asset_record_in(&tx, input).await?;
    tx.commit().await?;
    Ok(asset)
}

async fn ensure_canvas_image_media_asset_record_in<C>(
    db: &C,
    input: CanvasImageMediaAssetEnsureInput,
) -> Result<media_asset::Model, CanvasDbError>
where
    C: ConnectionTrait,
{
    let expected_chapter_id = input.image.chapter_id;
    let source_context =
        canvas_media_source_context_for_update(db, input.project_id, expected_chapter_id).await?;
    let image =
        canvas_generated_image_for_media_ensure(db, input.project_id, input.image.id).await?;
    if image.chapter_id != expected_chapter_id {
        return Err(CanvasDbError::InvalidArgument(format!(
            "图片 {} 的章节归属在媒体登记期间发生变化，请重试",
            image.id
        )));
    }
    let file_path = image.file_path.trim();
    if file_path.is_empty() {
        return Err(CanvasDbError::InvalidArgument(format!(
            "图片缺少文件路径，不能登记为媒体资产: {}",
            image.id
        )));
    }
    let asset = upsert_canvas_media_asset_record(
        db,
        CanvasMediaAssetUpsertInput {
            project_id: input.project_id,
            media_kind: "image",
            origin: input.origin,
            file_path: file_path.to_string(),
            thumbnail_path: None,
            width: Some(image.width),
            height: Some(image.height),
            duration_secs: None,
            fps: None,
            file_size: image.file_size,
        },
        &source_context,
    )
    .await?;

    upsert_canvas_image_media_provenance(db, &asset, &image, input.generation_spec_id).await?;
    ensure_canvas_media_binding(
        db,
        asset.id,
        "generated_image",
        image.id,
        "generated_result",
    )
    .await?;
    Ok(asset)
}
