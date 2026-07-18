use a4n_db::inspiration::{InspirationDbError, InspirationProjectContextRows};
use a4n_llm::provider::ChatRequest;

use crate::OrchestratorContext;
use a4n_core::error::AppError;
use a4n_core::models::user::message;

mod checkpoint;
mod context;
mod execution;
mod persistence;
mod request;
mod validation;

pub use request::build_inspiration_request_from_context;

#[cfg(test)]
mod tests;

pub struct InspirationStreamInput<'a, F>
where
    F: FnMut(&str, u32),
{
    pub project_id: i64,
    pub user_input: &'a str,
    pub request: ChatRequest,
    pub workflow_run_id: Option<i64>,
    pub on_chunk: F,
}

impl OrchestratorContext {
    pub async fn inspiration_messages(
        &self,
        project_id: i64,
        limit: u64,
    ) -> Result<Vec<message::Model>, AppError> {
        a4n_db::inspiration::inspiration_messages(self.db.connection(), project_id, limit)
            .await
            .map_err(inspiration_db_error_to_app)
    }

    pub async fn inspiration_project_context_rows(
        &self,
        project_id: i64,
    ) -> Result<InspirationProjectContextRows, AppError> {
        a4n_db::inspiration::inspiration_project_context_rows(self.db.connection(), project_id)
            .await
            .map_err(inspiration_db_error_to_app)
    }
}

fn inspiration_db_error_to_app(error: InspirationDbError) -> AppError {
    match error {
        InspirationDbError::NotFound(message) => AppError::NotFound(message),
        InspirationDbError::InvalidArgument(message) => AppError::InvalidArgument(message),
        InspirationDbError::Database(error) => AppError::Database(error),
    }
}
