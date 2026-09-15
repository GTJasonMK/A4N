use std::collections::HashMap;

use a4n_error::AppError;
use a4n_llm_provider_contract::provider::ChatRequest;
use serde_json::Value;

use a4n_prompt::rendering::PromptRenderer;

use super::super::super::OrchestratorContext;
use super::preparation::prepare_beat_mapping;
use super::request::build_beat_mapping_request;

impl OrchestratorContext {
    pub(in crate::outline) async fn preview_beat_mapping_request(
        &self,
        project_id: i64,
        part_id: i64,
        beat_sheet: &str,
        prompt_renderer: &dyn PromptRenderer,
        variables: &HashMap<String, Value>,
    ) -> Result<ChatRequest, AppError> {
        let prepared = prepare_beat_mapping(self, project_id, part_id, beat_sheet).await?;
        build_beat_mapping_request(project_id, &prepared, prompt_renderer, variables).await
    }
}
