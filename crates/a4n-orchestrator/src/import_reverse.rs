pub use a4n_db::import_reverse::{
    ImportReverseDbError, ReverseBlueprintApplyDraft, ReverseBlueprintApplyPlanRecord,
    ReverseBlueprintApplyRecord, ReverseBlueprintApplyResult, ReverseBlueprintContextRows,
    ReverseContextRows, ReverseOutlineApplyRow, ReversePartOutlineApplyRow,
};

mod application;
mod context;
mod execution;
pub mod request;

pub use context::resolve_reverse_content_sample;

use crate::OrchestratorContext;
use a4n_core::error::AppError;

impl OrchestratorContext {
    pub async fn reverse_context_rows(
        &self,
        project_id: i64,
        include_blueprint_context: bool,
    ) -> Result<ReverseContextRows, AppError> {
        a4n_db::import_reverse::reverse_context_rows(
            self.db.connection(),
            project_id,
            include_blueprint_context,
        )
        .await
        .map_err(import_reverse_db_error)
    }
}

fn import_reverse_db_error(error: ImportReverseDbError) -> AppError {
    match error {
        ImportReverseDbError::Database(error) => AppError::Database(error),
        ImportReverseDbError::NotFound(message) => AppError::NotFound(message),
        ImportReverseDbError::InvalidArgument(message) => AppError::InvalidArgument(message),
    }
}
