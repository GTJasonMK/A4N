use a4n_db::import_reverse::ImportReverseDbError;

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

/// Runtime-facing reverse-engineering request-preview boundary.
#[async_trait::async_trait]
pub trait ImportReversePreviewApplicationPort: Send + Sync {
    async fn prepare_request(
        &self,
        input: a4n_api_contract::import_reverse::ReverseEngineerInput,
        prompt_renderer: &dyn a4n_prompt::rendering::PromptRenderer,
    ) -> Result<a4n_llm::provider::ChatRequest, a4n_error::AppError>;
}

/// Runtime-facing reverse-engineering execution boundary.
pub trait ImportReverseGenerationApplicationPort: Send + Sync {
    fn run<'a>(
        &'a self,
        input: a4n_api_contract::import_reverse::ReverseEngineerInput,
        prompt_renderer: &'a dyn a4n_prompt::rendering::PromptRenderer,
        progress: &'a (
                dyn for<'b> Fn(u32, u32, &'b str) -> Result<(), a4n_error::AppError> + Send + Sync
            ),
        active_task_id: Option<&'a str>,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<
                    Output = Result<
                        a4n_api_contract::import_reverse::ReverseEngineeringCandidate,
                        a4n_error::AppError,
                    >,
                > + Send
                + 'a,
        >,
    >;
}

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
