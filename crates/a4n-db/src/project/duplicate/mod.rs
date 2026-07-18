mod derived;
mod freeform;
mod shared;
mod text;

use sea_orm::DbErr;
use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum ProjectMediaCopyError {
    #[error("媒体源路径无效：{0}")]
    InvalidSourcePath(String),
    #[error("媒体文件缺少扩展名: {path}")]
    MissingExtension { path: String },
    #[error("无法创建媒体复制目录 {}: {source}", path.display())]
    CreateDirectory {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error(
        "复制媒体文件失败: {} -> {}: {source}",
        source_path.display(),
        target_path.display()
    )]
    CopyFile {
        source_path: PathBuf,
        target_path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}

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
    #[error(transparent)]
    Serialization(#[from] serde_json::Error),
}

pub use derived::{DerivedProjectMediaFileCopier, duplicate_derived_project_record};
pub use freeform::{FreeformProjectMediaFileCopier, duplicate_freeform_project_record};
pub use text::{branch_text_project_record, duplicate_text_project_record};
