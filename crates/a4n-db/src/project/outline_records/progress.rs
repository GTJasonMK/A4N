use a4n_core::models::chapter::{chapter_outline, part_outline};
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder};

use super::contracts::{OutlineGenerationProgressRows, ProjectOutlineRecordError};

pub async fn outline_generation_progress_rows(
    db: &DatabaseConnection,
    project_id: i64,
) -> Result<OutlineGenerationProgressRows, ProjectOutlineRecordError> {
    let parts = part_outline::Entity::find()
        .filter(part_outline::Column::ProjectId.eq(project_id))
        .order_by_asc(part_outline::Column::PartNumber)
        .order_by_asc(part_outline::Column::Id)
        .all(db)
        .await
        .map_err(ProjectOutlineRecordError::Database)?;
    let chapter_outlines = chapter_outline::Entity::find()
        .filter(chapter_outline::Column::ProjectId.eq(project_id))
        .order_by_asc(chapter_outline::Column::ChapterNumber)
        .order_by_asc(chapter_outline::Column::Id)
        .all(db)
        .await
        .map_err(ProjectOutlineRecordError::Database)?;

    Ok(OutlineGenerationProgressRows {
        parts,
        chapter_outlines,
    })
}
