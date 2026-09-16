use a4n_db::import_reverse::ImportReverseDbError;
use a4n_novel_host_contract::{
    ImportReverseGenerationApplicationPort, ImportReversePreviewApplicationPort,
};

mod application;
mod candidate;
mod context;
mod execution;
mod json;
mod request;
mod target;
mod workflow_rules;

pub(crate) use target::ReverseEngineeringTarget;
pub(in crate::import_reverse) use workflow_rules::{
    IMPORT_REVERSE_WORKFLOW_KIND, IMPORT_REVERSE_WORKFLOW_MODE,
};

pub(crate) fn import_reverse_db_error(error: ImportReverseDbError) -> a4n_error::AppError {
    match error {
        ImportReverseDbError::Database(error) => a4n_error::AppError::Database(error),
        ImportReverseDbError::NotFound(message) => a4n_error::AppError::NotFound(message),
        ImportReverseDbError::InvalidArgument(message) => {
            a4n_error::AppError::InvalidArgument(message)
        }
        ImportReverseDbError::Internal(message) => {
            a4n_error::AppError::Internal(anyhow::anyhow!(message))
        }
    }
}
