use std::collections::HashMap;

use a4n_core::error::AppError;
use a4n_core::models::chapter::part_outline;
use chrono::Utc;
use serde_json::Value;

use a4n_prompt::rendering::PromptRenderer;

use super::super::super::OrchestratorContext;
use super::super::super::part_contract::{
    update_part_outline_and_rebind_chapters_transaction, update_part_outline_from_json,
};
use super::super::contracts::PartOutlineJsonInput;
use super::preparation::{
    PreparedPartRegeneration, prepare_last_part_regeneration, prepare_part_regeneration_by_id,
};

impl OrchestratorContext {
    pub async fn regenerate_last_part(
        &self,
        project_id: i64,
        prompt_renderer: &dyn PromptRenderer,
        variables: &HashMap<String, Value>,
        workflow_run_id: Option<i64>,
    ) -> Result<part_outline::Model, AppError> {
        let prepared = prepare_last_part_regeneration(self, project_id).await?;
        self.execute_part_regeneration(prepared, prompt_renderer, variables, workflow_run_id)
            .await
    }

    /// 重新生成指定分部。
    pub async fn regenerate_part_outline(
        &self,
        project_id: i64,
        part_id: i64,
        prompt_renderer: &dyn PromptRenderer,
        variables: &HashMap<String, Value>,
        workflow_run_id: Option<i64>,
    ) -> Result<part_outline::Model, AppError> {
        let prepared = prepare_part_regeneration_by_id(self, project_id, part_id).await?;
        self.execute_part_regeneration(prepared, prompt_renderer, variables, workflow_run_id)
            .await
    }

    async fn execute_part_regeneration(
        &self,
        prepared: PreparedPartRegeneration,
        prompt_renderer: &dyn PromptRenderer,
        variables: &HashMap<String, Value>,
        workflow_run_id: Option<i64>,
    ) -> Result<part_outline::Model, AppError> {
        let generated = self
            .generate_part_outline_json(PartOutlineJsonInput {
                project_id: prepared.project_id,
                part_num: prepared.target.part_number,
                start_ch: prepared.target.start_chapter,
                end_ch: prepared.target.end_chapter,
                chapters_per_part: prepared.context.chapters_per_part,
                requested_part_count: 1,
                blueprint_context: &prepared.context.blueprint_payload,
                previous_parts: &prepared.previous_parts,
                prompt_renderer,
                variables,
                workflow_run_id,
            })
            .await?;
        let now = Utc::now().naive_utc();
        let updated = if prepared.rebind_chapter_outlines {
            update_part_outline_and_rebind_chapters_transaction(
                &self.db,
                prepared.project_id,
                prepared.target.clone(),
                prepared.target.part_number,
                prepared.target.start_chapter,
                prepared.target.end_chapter,
                &generated.data,
                now,
            )
            .await?
        } else {
            update_part_outline_from_json(
                self.db.connection(),
                prepared.target.clone(),
                prepared.target.part_number,
                prepared.target.start_chapter,
                prepared.target.end_chapter,
                &generated.data,
                now,
            )
            .await?
        };
        self.record_part_outline_generation_run(&updated, &generated, "重新生成分部大纲")
            .await?;
        Ok(updated)
    }
}
