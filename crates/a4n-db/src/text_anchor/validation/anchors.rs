use a4n_core::models::project::text_anchor;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder};

use super::project::ensure_text_root_project;
use super::rules::{normalize_anchor_ids, validate_loaded_anchor_records};
use super::snapshot::validate_snapshot_filters;
use crate::text_anchor::TextAnchorDbResult;

pub async fn validate_anchor_ref_records<C>(
    db: &C,
    root_project_id: i64,
    source_snapshot_id: Option<i64>,
    source_snapshot_chapter_ids: &[i64],
    anchor_ids: &[i64],
) -> TextAnchorDbResult<Vec<text_anchor::Model>>
where
    C: ConnectionTrait,
{
    ensure_text_root_project(db, root_project_id).await?;
    if let Some(snapshot_id) = source_snapshot_id {
        validate_snapshot_filters(db, root_project_id, Some(snapshot_id), None).await?;
    }
    let unique_ids = normalize_anchor_ids(anchor_ids)?;
    let anchors = text_anchor::Entity::find()
        .filter(text_anchor::Column::Id.is_in(unique_ids.iter().copied()))
        .order_by_asc(text_anchor::Column::Id)
        .all(db)
        .await?;
    validate_loaded_anchor_records(
        &anchors,
        unique_ids.len(),
        root_project_id,
        source_snapshot_id,
        source_snapshot_chapter_ids,
    )?;
    Ok(anchors)
}
