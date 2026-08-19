use super::{WORKFLOW_STATUS_COMPLETED, types::CanvasMediaSourceContext};
use crate::canvas::CanvasDbError;
use a4n_core::models::media::media_asset;
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ConnectionTrait, Set};

mod binding;
mod context;
mod image;
mod lookup;
mod provenance;
mod video;

pub use image::ensure_canvas_image_media_asset_record;
pub use lookup::canvas_media_asset_by_id;
pub use video::ensure_canvas_video_media_asset_record;

pub(super) use binding::ensure_canvas_media_binding;
pub(super) use video::ensure_canvas_video_media_asset_record_in;

pub(super) struct CanvasMediaAssetUpsertInput {
    pub project_id: i64,
    pub media_kind: &'static str,
    pub origin: String,
    pub file_path: String,
    pub thumbnail_path: Option<String>,
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub duration_secs: Option<f32>,
    pub fps: Option<f32>,
    pub file_size: Option<i64>,
}

pub(super) async fn upsert_canvas_media_asset_record<C>(
    db: &C,
    input: CanvasMediaAssetUpsertInput,
    source_context: &CanvasMediaSourceContext,
) -> Result<media_asset::Model, CanvasDbError>
where
    C: ConnectionTrait,
{
    let now = Utc::now().naive_utc();
    let existing =
        lookup::canvas_media_asset_by_path_for_update(db, input.project_id, &input.file_path)
            .await?;
    if let Some(existing) = existing {
        let mut active: media_asset::ActiveModel = existing.into();
        active.root_project_id = Set(source_context.root_project_id);
        active.source_snapshot_id = Set(source_context.source_snapshot_id);
        active.source_chapter_id = Set(source_context.source_chapter_id);
        active.source_snapshot_chapter_id = Set(source_context.source_snapshot_chapter_id);
        active.media_kind = Set(input.media_kind.to_string());
        active.origin = Set(input.origin);
        active.file_path = Set(Some(input.file_path));
        active.thumbnail_path = Set(input.thumbnail_path);
        active.width = Set(input.width);
        active.height = Set(input.height);
        active.duration_secs = Set(input.duration_secs);
        active.fps = Set(input.fps);
        active.file_size = Set(input.file_size);
        active.status = Set(WORKFLOW_STATUS_COMPLETED.to_string());
        active.error_message = Set(None);
        active.updated_at = Set(now);
        return Ok(active.update(db).await?);
    }

    Ok(media_asset::ActiveModel {
        project_id: Set(input.project_id),
        root_project_id: Set(source_context.root_project_id),
        source_snapshot_id: Set(source_context.source_snapshot_id),
        source_chapter_id: Set(source_context.source_chapter_id),
        source_snapshot_chapter_id: Set(source_context.source_snapshot_chapter_id),
        media_kind: Set(input.media_kind.to_string()),
        origin: Set(input.origin),
        file_path: Set(Some(input.file_path)),
        thumbnail_path: Set(input.thumbnail_path),
        width: Set(input.width),
        height: Set(input.height),
        duration_secs: Set(input.duration_secs),
        fps: Set(input.fps),
        file_size: Set(input.file_size),
        checksum: Set(None),
        status: Set(WORKFLOW_STATUS_COMPLETED.to_string()),
        error_message: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await?)
}
