use std::collections::HashMap;

use a4n_core::error::AppError;
use a4n_core::models::chapter::part_outline;
use a4n_llm::provider::ChatRequest;
use chrono::Utc;

use crate::outline::context::load_part_generation_context;
use crate::outline::part_contract::{
    ensure_continue_part_request, ensure_no_part_outline_conflict, insert_part_outline_from_json,
    validate_part_outline_request,
};
use crate::outline::{OrchestratorContext, PartOutlineContinuationInput};
use a4n_prompt::rendering::PromptRenderer;

use super::contracts::{PartOutlineJsonInput, PartOutlineRequestInput};
use super::requests::build_part_outline_request;

impl OrchestratorContext {
    pub async fn continue_part_outline(
        &self,
        input: PartOutlineContinuationInput<'_>,
    ) -> Result<part_outline::Model, AppError> {
        let PartOutlineContinuationInput {
            project_id,
            part_number,
            start_chapter,
            end_chapter,
            prompt_renderer,
            variables,
            workflow_run_id,
        } = input;
        let context =
            load_part_generation_context(self.database.connection(), project_id, None, None)
                .await?;
        validate_part_outline_request(part_number, start_chapter, end_chapter)?;
        ensure_continue_part_request(&context.parts, part_number, start_chapter)?;
        ensure_no_part_outline_conflict(
            &context.parts,
            part_number,
            start_chapter,
            end_chapter,
            None,
        )?;

        let generated = self
            .generate_part_outline_json(PartOutlineJsonInput {
                project_id,
                part_num: part_number,
                start_ch: start_chapter,
                end_ch: end_chapter,
                chapters_per_part: context.chapters_per_part,
                requested_part_count: 1,
                blueprint_context: &context.blueprint_payload,
                previous_parts: &context.parts,
                prompt_renderer,
                variables,
                workflow_run_id,
            })
            .await?;
        let now = Utc::now().naive_utc();
        let part = insert_part_outline_from_json(
            self.database.connection(),
            project_id,
            part_number,
            start_chapter,
            end_chapter,
            &generated.data,
            now,
        )
        .await?;
        self.record_part_outline_generation_run(&part, &generated, "续写分部大纲")
            .await?;
        Ok(part)
    }

    pub async fn preview_continue_part_outline_request(
        &self,
        project_id: i64,
        part_number: i32,
        start_chapter: i32,
        end_chapter: i32,
        prompt_renderer: &dyn PromptRenderer,
        variables: &HashMap<String, serde_json::Value>,
    ) -> Result<ChatRequest, AppError> {
        let context =
            load_part_generation_context(self.database.connection(), project_id, None, None)
                .await?;
        validate_part_outline_request(part_number, start_chapter, end_chapter)?;
        ensure_continue_part_request(&context.parts, part_number, start_chapter)?;
        ensure_no_part_outline_conflict(
            &context.parts,
            part_number,
            start_chapter,
            end_chapter,
            None,
        )?;
        build_part_outline_request(PartOutlineRequestInput {
            project_id,
            part_num: part_number,
            start_ch: start_chapter,
            end_ch: end_chapter,
            chapters_per_part: context.chapters_per_part,
            requested_part_count: 1,
            blueprint_context: &context.blueprint_payload,
            previous_parts: &context.parts,
            prompt_renderer,
            variables,
        })
        .await
    }
}
