use super::{
    super::ProjectMutationError,
    hashing::{hash_json, to_json},
    types::DerivedChapterScope,
};
use a4n_core::models::{
    chapter::{chapter_outline, part_outline},
    project::source_snapshot_outline,
};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder, Set,
};
use serde::Serialize;

pub(super) async fn freeze_outlines<C>(
    db: &C,
    snapshot_id: i64,
    root_project_id: i64,
    scope: DerivedChapterScope,
) -> Result<(), ProjectMutationError>
where
    C: ConnectionTrait,
{
    let part_outlines = part_outline::Entity::find()
        .filter(part_outline::Column::ProjectId.eq(root_project_id))
        .filter(part_outline::Column::EndChapter.gte(scope.start_chapter))
        .filter(part_outline::Column::StartChapter.lte(scope.end_chapter))
        .order_by_asc(part_outline::Column::PartNumber)
        .all(db)
        .await
        .map_err(ProjectMutationError::Database)?;
    for outline in part_outlines {
        insert_snapshot_outline(
            db,
            snapshot_id,
            root_project_id,
            outline.id,
            "part",
            &outline,
        )
        .await?;
    }

    let chapter_outlines = chapter_outline::Entity::find()
        .filter(chapter_outline::Column::ProjectId.eq(root_project_id))
        .filter(chapter_outline::Column::ChapterNumber.gte(scope.start_chapter))
        .filter(chapter_outline::Column::ChapterNumber.lte(scope.end_chapter))
        .order_by_asc(chapter_outline::Column::ChapterNumber)
        .all(db)
        .await
        .map_err(ProjectMutationError::Database)?;
    for outline in chapter_outlines {
        insert_snapshot_outline(
            db,
            snapshot_id,
            root_project_id,
            outline.id,
            "chapter",
            &outline,
        )
        .await?;
    }
    Ok(())
}

async fn insert_snapshot_outline<C, T>(
    db: &C,
    snapshot_id: i64,
    root_project_id: i64,
    source_outline_id: i64,
    outline_kind: &str,
    outline: &T,
) -> Result<(), ProjectMutationError>
where
    C: ConnectionTrait,
    T: Serialize,
{
    let payload = to_json(outline)?;
    source_snapshot_outline::ActiveModel {
        snapshot_id: Set(snapshot_id),
        root_project_id: Set(root_project_id),
        source_outline_id: Set(source_outline_id),
        outline_kind: Set(outline_kind.to_string()),
        payload_hash: Set(hash_json(&payload)),
        payload_json: Set(payload),
        ..Default::default()
    }
    .insert(db)
    .await
    .map_err(ProjectMutationError::Database)?;
    Ok(())
}
