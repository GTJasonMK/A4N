pub use a4n_db::import_reverse::{
    ImportReverseDbError, ReverseBlueprintApplyDraft, ReverseBlueprintApplyPlanRecord,
    ReverseBlueprintApplyRecord, ReverseBlueprintApplyResult, ReverseBlueprintContextRows,
    ReverseContextRows, ReverseOutlineApplyRow, ReversePartOutlineApplyRow,
};

mod application;
mod candidate;
mod context;
mod execution;
pub mod request;
mod target;
mod workflow_rules;

pub use application::{
    ReverseEngineerApplicationInput, ReverseEngineerPreviewInput, ReverseProgressCallback,
};
pub use context::resolve_reverse_content_sample;
pub use target::ReverseEngineeringTarget;
pub(crate) use target::ReverseEngineeringTargetDomain;
pub(crate) use workflow_rules::{IMPORT_REVERSE_WORKFLOW_KIND, IMPORT_REVERSE_WORKFLOW_MODE};
pub use workflow_rules::{
    import_reverse_business_key, import_reverse_workflow_fingerprint,
    import_reverse_workflow_run_input,
};

pub(crate) fn import_reverse_db_error(error: ImportReverseDbError) -> a4n_core::error::AppError {
    match error {
        ImportReverseDbError::Database(error) => a4n_core::error::AppError::Database(error),
        ImportReverseDbError::NotFound(message) => a4n_core::error::AppError::NotFound(message),
        ImportReverseDbError::InvalidArgument(message) => {
            a4n_core::error::AppError::InvalidArgument(message)
        }
        ImportReverseDbError::Internal(message) => {
            a4n_core::error::AppError::Internal(anyhow::anyhow!(message))
        }
    }
}
