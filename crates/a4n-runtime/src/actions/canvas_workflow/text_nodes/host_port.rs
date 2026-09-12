use a4n_error::AppError;
use a4n_orchestrator::canvas::{CanvasTextGenerationLlmCall, CanvasTextGenerationLlmPort};
use async_trait::async_trait;

use crate::AppState;

pub(super) struct RuntimeCanvasTextGenerationLlmPort<'a> {
    state: &'a AppState,
}

impl<'a> RuntimeCanvasTextGenerationLlmPort<'a> {
    pub(super) fn new(state: &'a AppState) -> Self {
        Self { state }
    }
}

#[async_trait]
impl CanvasTextGenerationLlmPort for RuntimeCanvasTextGenerationLlmPort<'_> {
    async fn max_output_tokens(&self, config_id: Option<i64>) -> Result<usize, AppError> {
        Ok(self
            .state
            .dynamic_llm_provider_for_profile()?
            .capabilities_for_config_id(config_id)
            .await?
            .max_output_tokens)
    }

    async fn stream_complete(
        &self,
        input: CanvasTextGenerationLlmCall,
    ) -> Result<a4n_llm::provider::ChatResponse, AppError> {
        self.state
            .dynamic_llm_provider_for_profile()?
            .chat_with_config_id(input.config_id, input.request)
            .await
    }
}
