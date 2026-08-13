use sea_orm::{ConnectionTrait, DbErr};

mod prompt_records;
pub use prompt_records::*;

mod generated_images;
pub use generated_images::*;

mod generated_videos;
pub use generated_videos::*;

mod segment_plan_draft;
pub use segment_plan_draft::*;

mod timeline;
pub use timeline::*;

#[derive(Debug, thiserror::Error)]
pub enum VideoDbError {
    #[error("{0}")]
    InvalidArgument(String),
    #[error("当前状态 {current} 不允许执行 {operation}")]
    InvalidState { current: String, operation: String },
    #[error("{0}")]
    NotFound(String),
    #[error("{0}")]
    Forbidden(String),
    #[error(transparent)]
    Database(#[from] DbErr),
}

impl From<crate::canvas::CanvasDbError> for VideoDbError {
    fn from(err: crate::canvas::CanvasDbError) -> Self {
        match err {
            crate::canvas::CanvasDbError::InvalidArgument(message) => {
                VideoDbError::InvalidArgument(message)
            }
            crate::canvas::CanvasDbError::NotFound(message) => VideoDbError::NotFound(message),
            crate::canvas::CanvasDbError::Forbidden(message) => VideoDbError::Forbidden(message),
            crate::canvas::CanvasDbError::Database(err) => VideoDbError::Database(err),
        }
    }
}

impl From<crate::generation::GenerationDbError> for VideoDbError {
    fn from(err: crate::generation::GenerationDbError) -> Self {
        match err {
            crate::generation::GenerationDbError::InvalidArgument(message) => {
                VideoDbError::InvalidArgument(message)
            }
            crate::generation::GenerationDbError::NotFound(message) => {
                VideoDbError::NotFound(message)
            }
            crate::generation::GenerationDbError::Database(err) => VideoDbError::Database(err),
        }
    }
}

async fn lock_generated_media_project<C>(db: &C, project_id: i64) -> Result<(), VideoDbError>
where
    C: ConnectionTrait,
{
    let project_id = validate_positive_video_i64(project_id, "project_id")?;
    let project = crate::project::project_by_id_for_update(db, project_id)
        .await?
        .ok_or_else(|| VideoDbError::NotFound(format!("项目不存在: {project_id}")))?;
    if project.lifecycle_gate != crate::generation::PROJECT_GATE_ACTIVE {
        return Err(VideoDbError::InvalidState {
            current: project.lifecycle_gate,
            operation: "写入生成媒体".to_string(),
        });
    }
    Ok(())
}

pub(super) fn required_video_record_text(value: &str, field: &str) -> Result<String, VideoDbError> {
    let value = value.trim();
    if value.is_empty() {
        return Err(VideoDbError::InvalidArgument(format!("{field} 不能为空")));
    }
    Ok(value.to_string())
}

pub(super) fn normalize_optional_video_record_text(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
}

pub(super) fn validate_positive_video_i64(value: i64, field: &str) -> Result<i64, VideoDbError> {
    if value <= 0 {
        return Err(VideoDbError::InvalidArgument(format!("{field} 必须大于 0")));
    }
    Ok(value)
}

pub(super) fn validate_positive_video_i32(value: i32, field: &str) -> Result<i32, VideoDbError> {
    if value <= 0 {
        return Err(VideoDbError::InvalidArgument(format!("{field} 必须大于 0")));
    }
    Ok(value)
}

pub(super) fn validate_positive_video_ids(values: &[i64], field: &str) -> Result<(), VideoDbError> {
    if let Some(value) = values.iter().find(|value| **value <= 0) {
        return Err(VideoDbError::InvalidArgument(format!(
            "{field} 必须全部大于 0: {value}"
        )));
    }
    Ok(())
}

mod source_context;
pub use source_context::*;

#[cfg(test)]
mod tests;
