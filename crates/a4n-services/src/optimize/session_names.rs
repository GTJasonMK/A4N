use a4n_core::error::AppError;
use a4n_core::models::ai::optimization_session;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};

fn required_session_name(name: &str) -> Result<String, AppError> {
    let name = name.trim();
    if name.is_empty() {
        return Err(AppError::InvalidArgument("会话名称不能为空".to_string()));
    }
    Ok(name.to_string())
}

pub(super) async fn required_available_session_name(
    db: &DatabaseConnection,
    project_id: i64,
    chapter_id: i64,
    name: &str,
) -> Result<String, AppError> {
    let name = required_session_name(name)?;
    let exists = optimization_session::Entity::find()
        .filter(optimization_session::Column::ProjectId.eq(project_id))
        .filter(optimization_session::Column::ChapterId.eq(chapter_id))
        .filter(optimization_session::Column::Name.eq(name.as_str()))
        .one(db)
        .await
        .map_err(AppError::Database)?
        .is_some();
    if exists {
        return Err(AppError::InvalidArgument(format!(
            "本章已存在名为「{name}」的内容优化会话"
        )));
    }
    Ok(name)
}
