use a4n_core::models::project::novel_project;
use sea_orm::{DatabaseConnection, EntityTrait};

use crate::text_anchor::{PROJECT_KIND_TEXT, TextAnchorDbError, TextAnchorDbResult};

pub(in crate::text_anchor) async fn ensure_text_root_project(
    db: &DatabaseConnection,
    project_id: i64,
) -> TextAnchorDbResult<novel_project::Model> {
    let project = novel_project::Entity::find_by_id(project_id)
        .one(db)
        .await?
        .ok_or_else(|| TextAnchorDbError::NotFound(format!("文本根项目不存在: {project_id}")))?;
    if project.project_kind != PROJECT_KIND_TEXT {
        return Err(TextAnchorDbError::InvalidArgument(format!(
            "项目不是文本根项目: {project_id}"
        )));
    }
    Ok(project)
}
