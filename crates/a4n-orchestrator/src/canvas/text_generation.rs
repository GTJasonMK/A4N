mod contracts;
mod execution;
mod input;
mod request;

use a4n_api_contract::canvas::CanvasRunSummary;
use a4n_canvas_contract::CanvasLlmTextNodeApplicationInput;
use a4n_error::AppError;
use async_trait::async_trait;

pub use contracts::{
    CanvasTextGenerationLlmCall, CanvasTextGenerationLlmPort, CanvasTextNodeExecutionInput,
};
pub use request::build_canvas_llm_text_request_for_preview;

#[async_trait]
pub trait CanvasLlmTextNodeApplicationPort: Send + Sync {
    async fn run_llm_text(
        &self,
        input: CanvasLlmTextNodeApplicationInput,
        llm_port: &dyn CanvasTextGenerationLlmPort,
    ) -> Result<CanvasRunSummary, AppError>;
}

#[cfg(test)]
mod tests;
