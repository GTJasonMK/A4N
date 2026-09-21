//! 大纲分部/章节大纲的 DB 边界 helper。

mod chapters;
mod generation_sources;
mod parts;
mod ranges;
mod types;

pub use chapters::{
    insert_chapter_outline_record, sync_chapter_outline_part_assignments,
    sync_chapter_outline_records_by_number, sync_chapter_outline_records_by_number_transaction,
    update_chapter_outline_record,
};
pub use generation_sources::{
    OUTLINE_GENERATION_SOURCE_CHANGED, OutlineGenerationSourceRows,
    load_outline_generation_source_rows, load_outline_generation_source_rows_for_update,
    lock_outline_generation_sources, outline_generation_source_snapshot,
};
pub use parts::{
    insert_part_outline_record, sync_part_outline_records,
    sync_part_outline_records_and_assign_chapters_in, update_part_outline_and_rebind_chapters_in,
    update_part_outline_and_rebind_chapters_transaction, update_part_outline_record,
};
pub use ranges::{
    chapter_outline_ids_in_part_range, chapter_outline_ids_in_part_ranges,
    part_id_for_chapter_number, validate_part_outline_records,
};
pub use types::{OutlineDbError, PartOutlineSyncDraft};

async fn lock_outline_project_for_write<C>(
    db: &C,
    project_id: i64,
    operation: &str,
) -> Result<a4n_db_models::models::project::novel_project::Model, OutlineDbError>
where
    C: sea_orm::ConnectionTrait,
{
    let project = crate::project::project_by_id_for_update(db, project_id)
        .await?
        .ok_or_else(|| OutlineDbError::InvalidArgument(format!("项目不存在: {project_id}")))?;
    if project.lifecycle_gate != crate::generation::PROJECT_GATE_ACTIVE {
        return Err(OutlineDbError::InvalidState {
            current: project.lifecycle_gate,
            operation: operation.to_string(),
        });
    }
    Ok(project)
}

#[cfg(test)]
mod tests;
