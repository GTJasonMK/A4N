mod derived;
mod freeform;
mod shared;
mod standalone_comic;
mod text;

use a4n_domain::project::ProjectMediaCopyError;
use sea_orm::DbErr;

#[derive(Debug, thiserror::Error)]
pub enum ProjectDuplicateError {
    #[error("{0}")]
    InvalidArgument(String),
    #[error("状态无效：当前状态 {current}，不允许操作 {operation}")]
    InvalidState { current: String, operation: String },
    #[error("媒体文件复制失败：{0}")]
    MediaCopy(#[from] ProjectMediaCopyError),
    #[error(transparent)]
    Database(#[from] DbErr),
}

pub use derived::duplicate_derived_project_record;
pub use freeform::duplicate_freeform_project_record;
pub use standalone_comic::duplicate_standalone_comic_project_record;
pub use text::duplicate_text_project_record;
