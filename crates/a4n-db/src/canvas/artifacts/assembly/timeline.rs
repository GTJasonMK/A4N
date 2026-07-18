use a4n_core::models::video_project::assembly_timeline;
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder, Set,
};

use crate::canvas::CanvasDbError;

use super::super::WORKFLOW_STATUS_COMPLETED;

pub(super) async fn upsert_canvas_assembly_timeline<C>(
    db: &C,
    project_id: i64,
    graph_id: i64,
    node_id: i64,
    preview_media_asset_id: i64,
) -> Result<assembly_timeline::Model, CanvasDbError>
where
    C: ConnectionTrait,
{
    let now = Utc::now().naive_utc();
    let cache_key = format!("canvas_graph:{graph_id}:node:{node_id}");
    let existing = assembly_timeline::Entity::find()
        .filter(assembly_timeline::Column::ProjectId.eq(project_id))
        .filter(assembly_timeline::Column::PreviewCacheKey.eq(cache_key.clone()))
        .order_by_asc(assembly_timeline::Column::Id)
        .one(db)
        .await?;
    if let Some(existing) = existing {
        let next_version = existing.version.checked_add(1).ok_or_else(|| {
            CanvasDbError::InvalidArgument("画布成片时间线 version 超出 i32 范围".to_string())
        })?;
        let mut active: assembly_timeline::ActiveModel = existing.into();
        active.source_chapter_id = Set(None);
        active.source_snapshot_chapter_id = Set(None);
        active.version = Set(next_version);
        active.status = Set(WORKFLOW_STATUS_COMPLETED.to_string());
        active.preview_media_asset_id = Set(Some(preview_media_asset_id));
        active.preview_cache_key = Set(Some(cache_key));
        active.updated_at = Set(now);
        return Ok(active.update(db).await?);
    }
    Ok(assembly_timeline::ActiveModel {
        project_id: Set(project_id),
        source_chapter_id: Set(None),
        source_snapshot_chapter_id: Set(None),
        version: Set(1),
        status: Set(WORKFLOW_STATUS_COMPLETED.to_string()),
        preview_media_asset_id: Set(Some(preview_media_asset_id)),
        preview_cache_key: Set(Some(cache_key)),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await?)
}
