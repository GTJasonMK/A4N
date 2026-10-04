//! LLM 上下文包领域私有 owner。
//!
//! 持有 `DbHandle`，封装 context pack 的 record-or-reuse 落库。

use a4n_db::DbHandle;
use a4n_db::llm_context::LlmContextPackDbError;
use a4n_domain::llm::ContextPackReference;
use a4n_domain::llm::RecordOrReuseContextPackInput;
use a4n_error::AppError;

#[derive(Clone)]
pub(super) struct LlmContextApplication {
    database: DbHandle,
}

impl LlmContextApplication {
    pub(super) fn new(database: DbHandle) -> Self {
        Self { database }
    }

    pub(super) async fn record_or_reuse_context_pack_reference(
        &self,
        input: RecordOrReuseContextPackInput,
    ) -> Result<ContextPackReference, AppError> {
        a4n_db::llm_context::record_or_reuse_context_pack_reference(self.database.executor(), input)
            .await
            .map_err(llm_context_pack_db_error)
    }

    pub(super) async fn record_or_reuse_context_pack_references(
        &self,
        inputs: Vec<RecordOrReuseContextPackInput>,
    ) -> Result<Vec<ContextPackReference>, AppError> {
        a4n_db::llm_context::record_or_reuse_context_pack_references(
            self.database.executor(),
            inputs,
        )
        .await
        .map_err(llm_context_pack_db_error)
    }
}

fn llm_context_pack_db_error(error: LlmContextPackDbError) -> AppError {
    match error {
        LlmContextPackDbError::InvalidArgument(message) => AppError::InvalidArgument(message),
        LlmContextPackDbError::Database(error) => AppError::database(error),
    }
}
