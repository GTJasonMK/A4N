use std::collections::HashMap;

use a4n_api_contract::outline::OutlineDeleteTarget;
use a4n_core::error::AppError;

use crate::outline::context::load_part_generation_context;
use crate::outline::part_contract::validate_existing_parts_for_plan;
use crate::outline::{
    OrchestratorContext, OutlineRequestPreviewStep, blocked_outline_preview_step,
    ready_outline_preview_step,
};
use a4n_prompt::rendering::PromptRenderer;

use super::super::contracts::PartOutlineRequestInput;
use super::super::planning::{
    plan_next_parts, plan_replacement_parts, validate_replacement_part_sources,
};
use super::super::requests::build_part_outline_request;

impl OrchestratorContext {
    pub async fn preview_generate_parts_requests(
        &self,
        project_id: i64,
        part_count: Option<i32>,
        chapters_per_part: Option<i32>,
        expected_parts: &[OutlineDeleteTarget],
        regenerate: bool,
        prompt_renderer: &dyn PromptRenderer,
        variables: &HashMap<String, serde_json::Value>,
    ) -> Result<Vec<OutlineRequestPreviewStep>, AppError> {
        let mut context = load_part_generation_context(
            self.database.connection(),
            project_id,
            part_count,
            chapters_per_part,
        )
        .await?;
        validate_existing_parts_for_plan(&context.parts)?;
        let batch = if regenerate {
            context.requested_part_count =
                validate_replacement_part_sources(&context.parts, expected_parts)?;
            plan_replacement_parts(context.requested_part_count, context.chapters_per_part)?
        } else {
            plan_next_parts(
                &context.parts,
                context.requested_part_count,
                context.chapters_per_part,
            )?
        };
        let num_parts = i32::try_from(batch.len()).map_err(|_| {
            AppError::InvalidArgument(format!("待预览分部数量超过 i32 范围：{}", batch.len()))
        })?;

        let mut steps = Vec::new();
        let previous_parts = if regenerate {
            Vec::new()
        } else {
            context.parts.clone()
        };
        let mut first_pending_seen = false;

        for (part_num, start_ch, end_ch) in batch {
            let title = format!("第 {part_num}/{num_parts} 部（第 {start_ch}-{end_ch} 章）");

            if first_pending_seen {
                steps.push(blocked_outline_preview_step(
                    format!("part_{part_num}"),
                    title,
                    "part_outline_single",
                    "该分部请求依赖前一部分部模型输出，需先完成前序阶段。",
                ));
                continue;
            }

            let request = build_part_outline_request(PartOutlineRequestInput {
                project_id,
                part_num,
                start_ch,
                end_ch,
                chapters_per_part: context.chapters_per_part,
                requested_part_count: context.requested_part_count,
                blueprint_context: &context.blueprint_payload,
                previous_parts: &previous_parts,
                source_snapshot: &context.source_snapshot,
                prompt_renderer,
                variables,
            })
            .await?;
            steps.push(ready_outline_preview_step(
                format!("part_{part_num}"),
                title,
                "part_outline_single",
                request,
            ));
            first_pending_seen = true;
        }

        Ok(steps)
    }
}
