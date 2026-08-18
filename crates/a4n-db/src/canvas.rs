mod artifacts;
mod graph_record;
mod references;
mod transfer;

pub use artifacts::*;
pub use graph_record::*;
pub use references::*;
pub use transfer::*;

use sea_orm::DbErr;

const PROJECT_KIND_VIDEO: &str = "video";

#[derive(Debug, thiserror::Error)]
pub enum CanvasDbError {
    #[error("{0}")]
    InvalidArgument(String),
    #[error("{0}")]
    NotFound(String),
    #[error("{0}")]
    Forbidden(String),
    #[error(transparent)]
    Database(#[from] DbErr),
}
