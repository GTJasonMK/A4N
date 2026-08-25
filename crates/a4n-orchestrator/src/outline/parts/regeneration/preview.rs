use std::collections::HashMap;

use a4n_core::error::AppError;
use a4n_llm::provider::ChatRequest;
use serde_json::Value;

use a4n_prompt::rendering::PromptRenderer;

use super::super::super::OrchestratorContext;
use super::super::contracts::PartOutlineRequestInput;
use super::super::requests::build_part_outline_request;
use super::preparation::{
    PreparedPartRegeneration, prepare_last_part_regeneration, prepare_part_regeneration_by_id,
};

impl OrchestratorContext {
    pub(in crate::outline) async fn preview_regenerate_last_part_request(
        &self,
        project_id: i64,
        prompt_renderer: &dyn PromptRenderer,
        variables: &HashMap<String, Value>,
    ) -> Result<ChatRequest, AppError> {
        let prepared = prepare_last_part_regeneration(self, project_id).await?;
        preview_regenerate_part_request(prepared, prompt_renderer, variables).await
    }

    pub(in crate::outline) async fn preview_regenerate_part_outline_request(
        &self,
        project_id: i64,
        part_id: i64,
        prompt_renderer: &dyn PromptRenderer,
        variables: &HashMap<String, Value>,
    ) -> Result<ChatRequest, AppError> {
        let prepared = prepare_part_regeneration_by_id(self, project_id, part_id).await?;
        preview_regenerate_part_request(prepared, prompt_renderer, variables).await
    }
}

async fn preview_regenerate_part_request(
    prepared: PreparedPartRegeneration,
    prompt_renderer: &dyn PromptRenderer,
    variables: &HashMap<String, Value>,
) -> Result<ChatRequest, AppError> {
    build_part_outline_request(PartOutlineRequestInput {
        project_id: prepared.project_id,
        part_num: prepared.target.part_number,
        start_ch: prepared.target.start_chapter,
        end_ch: prepared.target.end_chapter,
        chapters_per_part: prepared.context.chapters_per_part,
        requested_part_count: 1,
        blueprint_context: &prepared.context.blueprint_payload,
        previous_parts: &prepared.previous_parts,
        source_snapshot: &prepared.context.source_snapshot,
        prompt_renderer,
        variables,
    })
    .await
}
