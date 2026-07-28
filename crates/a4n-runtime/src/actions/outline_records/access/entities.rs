use crate::state::{CollaborationRuntimeFacade, ProjectOutlineRecordsRuntimeFacade};
use a4n_core::error::AppError;
use a4n_core::models::chapter::{chapter_outline, part_outline};
use a4n_orchestrator::collaboration::ProjectAccessRole;

pub(in crate::actions::outline_records) fn ensure_chapter_outline_belongs_to_project(
    outline: &chapter_outline::Model,
    project_id: i64,
) -> Result<(), AppError> {
    if outline.project_id == project_id {
        return Ok(());
    }
    Err(AppError::Forbidden(format!(
        "章节大纲 {} 不属于项目 {}",
        outline.id, project_id
    )))
}

pub(in crate::actions::outline_records) async fn ensure_chapter_outline_access(
    collaboration: &CollaborationRuntimeFacade<'_>,
    outline_records: &ProjectOutlineRecordsRuntimeFacade<'_>,
    user_id: i64,
    project_id: i64,
    outline_id: i64,
) -> Result<chapter_outline::Model, AppError> {
    collaboration
        .verify_project_permission(project_id, user_id, ProjectAccessRole::Editor)
        .await?;
    let outline = outline_records
        .chapter_outline_by_id(outline_id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("章节大纲不存在: {outline_id}")))?;
    ensure_chapter_outline_belongs_to_project(&outline, project_id)?;
    Ok(outline)
}

pub(in crate::actions::outline_records) async fn ensure_part_access(
    collaboration: &CollaborationRuntimeFacade<'_>,
    outline_records: &ProjectOutlineRecordsRuntimeFacade<'_>,
    user_id: i64,
    project_id: i64,
    part_id: i64,
) -> Result<part_outline::Model, AppError> {
    collaboration
        .verify_project_permission(project_id, user_id, ProjectAccessRole::Editor)
        .await?;
    let part = outline_records
        .part_outline_by_id(part_id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("分部大纲不存在: {part_id}")))?;
    if part.project_id != project_id {
        return Err(AppError::Forbidden(format!(
            "分部大纲 {part_id} 不属于项目 {project_id}"
        )));
    }
    Ok(part)
}
