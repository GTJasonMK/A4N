use a4n_db_models::models::chapter::{chapter_outline, part_outline};
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, QuerySelect};

use super::contracts::{
    OutlineGenerationProgressChapter, OutlineGenerationProgressPart, OutlineGenerationProgressRows,
    ProjectOutlineRecordError,
};

pub async fn outline_generation_progress_rows(
    db: &DatabaseConnection,
    project_id: i64,
) -> Result<OutlineGenerationProgressRows, ProjectOutlineRecordError> {
    let parts = part_outline::Entity::find()
        .select_only()
        .column(part_outline::Column::Id)
        .column(part_outline::Column::StartChapter)
        .column(part_outline::Column::EndChapter)
        .filter(part_outline::Column::ProjectId.eq(project_id))
        .order_by_asc(part_outline::Column::PartNumber)
        .order_by_asc(part_outline::Column::Id)
        .into_tuple::<(i64, i32, i32)>()
        .all(db)
        .await
        .map_err(ProjectOutlineRecordError::Database)?
        .into_iter()
        .map(
            |(id, start_chapter, end_chapter)| OutlineGenerationProgressPart {
                id,
                start_chapter,
                end_chapter,
            },
        )
        .collect();
    let chapter_outlines = chapter_outline::Entity::find()
        .select_only()
        .column(chapter_outline::Column::Id)
        .column(chapter_outline::Column::ChapterNumber)
        .column(chapter_outline::Column::Status)
        .filter(chapter_outline::Column::ProjectId.eq(project_id))
        .order_by_asc(chapter_outline::Column::ChapterNumber)
        .order_by_asc(chapter_outline::Column::Id)
        .into_tuple::<(i64, i32, String)>()
        .all(db)
        .await
        .map_err(ProjectOutlineRecordError::Database)?
        .into_iter()
        .map(
            |(id, chapter_number, status)| OutlineGenerationProgressChapter {
                id,
                chapter_number,
                status,
            },
        )
        .collect();

    Ok(OutlineGenerationProgressRows {
        parts,
        chapter_outlines,
    })
}
