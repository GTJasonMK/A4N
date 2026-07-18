pub use a4n_db::llm_context::{LlmContextPackDbError, RecordOrReuseContextPackInput};

use a4n_core::error::AppError;
use a4n_core::models::generation::llm_context_pack;

use crate::OrchestratorContext;

impl OrchestratorContext {
    pub async fn record_or_reuse_context_pack(
        &self,
        input: RecordOrReuseContextPackInput,
    ) -> Result<llm_context_pack::Model, AppError> {
        a4n_db::llm_context::record_or_reuse_context_pack(self.db.connection(), input)
            .await
            .map_err(llm_context_pack_db_error)
    }
}

fn llm_context_pack_db_error(error: LlmContextPackDbError) -> AppError {
    match error {
        LlmContextPackDbError::InvalidArgument(message) => AppError::InvalidArgument(message),
        LlmContextPackDbError::Database(error) => AppError::Database(error),
    }
}
