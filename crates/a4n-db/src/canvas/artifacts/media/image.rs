use super::super::{WORKFLOW_STATUS_COMPLETED, types::CanvasImageMediaAssetEnsureInput};
use super::{
    binding::ensure_canvas_media_binding,
    context::canvas_media_source_context_for_update,
    lookup::{canvas_generated_image_for_media_ensure, canvas_media_asset_by_path_for_update},
    provenance::upsert_canvas_image_media_provenance,
};
use crate::canvas::CanvasDbError;
use a4n_core::models::media::media_asset;
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ConnectionTrait, Set, TransactionTrait};

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
    let now = Utc::now().naive_utc();
    let existing = canvas_media_asset_by_path_for_update(db, input.project_id, file_path).await?;
    let asset = if let Some(existing) = existing {
        let mut active: media_asset::ActiveModel = existing.into();
        active.root_project_id = Set(source_context.root_project_id);
        active.source_snapshot_id = Set(source_context.source_snapshot_id);
        active.source_chapter_id = Set(source_context.source_chapter_id);
        active.source_snapshot_chapter_id = Set(source_context.source_snapshot_chapter_id);
        active.media_kind = Set("image".to_string());
        active.origin = Set(input.origin);
        active.file_path = Set(Some(file_path.to_string()));
        active.thumbnail_path = Set(None);
        active.width = Set(Some(image.width));
        active.height = Set(Some(image.height));
        active.duration_secs = Set(None);
        active.fps = Set(None);
        active.file_size = Set(image.file_size);
        active.status = Set(WORKFLOW_STATUS_COMPLETED.to_string());
        active.error_message = Set(None);
        active.updated_at = Set(now);
        active.update(db).await?
    } else {
        media_asset::ActiveModel {
            project_id: Set(input.project_id),
            root_project_id: Set(source_context.root_project_id),
            source_snapshot_id: Set(source_context.source_snapshot_id),
            source_chapter_id: Set(source_context.source_chapter_id),
            source_snapshot_chapter_id: Set(source_context.source_snapshot_chapter_id),
            media_kind: Set("image".to_string()),
            origin: Set(input.origin),
            file_path: Set(Some(file_path.to_string())),
            thumbnail_path: Set(None),
            width: Set(Some(image.width)),
            height: Set(Some(image.height)),
            duration_secs: Set(None),
            fps: Set(None),
            file_size: Set(image.file_size),
            checksum: Set(None),
            status: Set(WORKFLOW_STATUS_COMPLETED.to_string()),
            error_message: Set(None),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(db)
        .await?
    };

    upsert_canvas_image_media_provenance(db, &asset, &image, input.generation_spec_id, now).await?;
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
