//! 大纲分部/章节大纲的 DB 边界 helper。

mod chapters;
mod generation_sources;
mod parts;
mod ranges;
mod types;

pub use chapters::*;
pub use generation_sources::*;
pub use parts::*;
pub use ranges::{
    chapter_outline_ids_in_part_range, chapter_outline_ids_in_part_ranges,
    part_id_for_chapter_number, validate_part_outline_records,
};
pub use types::*;

async fn lock_outline_project_for_write<C>(
    db: &C,
    project_id: i64,
    operation: &str,
) -> Result<a4n_core::models::project::novel_project::Model, OutlineDbError>
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
