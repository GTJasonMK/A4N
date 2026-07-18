use super::super::{WORKFLOW_STATUS_COMPLETED, types::CanvasVideoMediaAssetEnsureInput};
use super::{
    binding::ensure_canvas_media_binding,
    context::canvas_media_source_context_for_update,
    lookup::{canvas_generated_video_for_media_ensure, canvas_media_asset_by_path_for_update},
    provenance::upsert_canvas_video_media_provenance,
};
use crate::canvas::CanvasDbError;
use a4n_core::models::media::media_asset;
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ConnectionTrait, Set, TransactionTrait};

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
    let now = Utc::now().naive_utc();
    let existing = canvas_media_asset_by_path_for_update(db, input.project_id, &file_path).await?;
    let asset = if let Some(existing) = existing {
        let mut active: media_asset::ActiveModel = existing.into();
        active.root_project_id = Set(source_context.root_project_id);
        active.source_snapshot_id = Set(source_context.source_snapshot_id);
        active.source_chapter_id = Set(source_context.source_chapter_id);
        active.source_snapshot_chapter_id = Set(source_context.source_snapshot_chapter_id);
        active.media_kind = Set("video".to_string());
        active.origin = Set(input.origin);
        active.file_path = Set(Some(file_path.clone()));
        active.thumbnail_path = Set(video.thumbnail_path.clone());
        active.width = Set(Some(video.width));
        active.height = Set(Some(video.height));
        active.duration_secs = Set(Some(video.duration_secs));
        active.fps = Set(Some(video.fps as f32));
        active.file_size = Set(video.file_size);
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
            media_kind: Set("video".to_string()),
            origin: Set(input.origin),
            file_path: Set(Some(file_path.clone())),
            thumbnail_path: Set(video.thumbnail_path.clone()),
            width: Set(Some(video.width)),
            height: Set(Some(video.height)),
            duration_secs: Set(Some(video.duration_secs)),
            fps: Set(Some(video.fps as f32)),
            file_size: Set(video.file_size),
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

    upsert_canvas_video_media_provenance(db, &asset, &video, input.generation_spec_id, now).await?;
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
