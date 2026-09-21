use a4n_api_contract::outline::OutlineReadApplicationPort;
use a4n_error::AppError;

#[cfg(test)]
use a4n_db_models::models::chapter::chapter_outline;

#[cfg(test)]
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
    outline_records: &dyn OutlineReadApplicationPort,
    user_id: i64,
    project_id: i64,
    outline_id: i64,
) -> Result<(), AppError> {
    outline_records
        .ensure_chapter_outline_access(user_id, project_id, outline_id)
        .await
}

pub(in crate::actions::outline_records) async fn ensure_part_access(
    outline_records: &dyn OutlineReadApplicationPort,
    user_id: i64,
    project_id: i64,
    part_id: i64,
) -> Result<(), AppError> {
    outline_records
        .ensure_part_access(user_id, project_id, part_id)
        .await
}
