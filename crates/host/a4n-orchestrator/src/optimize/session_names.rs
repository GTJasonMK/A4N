use a4n_error::AppError;
use sea_orm::ConnectionTrait;

use super::optimize_db_error;

fn required_session_name(name: &str) -> Result<String, AppError> {
    let name = name.trim();
    if name.is_empty() {
        return Err(AppError::InvalidArgument("会话名称不能为空".to_string()));
    }
    Ok(name.to_string())
}

pub(super) async fn required_available_session_name<C>(
    db: &C,
    project_id: i64,
    chapter_id: i64,
    name: &str,
) -> Result<String, AppError>
where
    C: ConnectionTrait,
{
    let name = required_session_name(name)?;
    let exists =
        a4n_db::optimize::optimization_session_name_exists(db, project_id, chapter_id, &name)
            .await
            .map_err(optimize_db_error)?;
    if exists {
        return Err(AppError::InvalidArgument(format!(
            "本章已存在名为「{name}」的内容优化会话"
        )));
    }
    Ok(name)
}
