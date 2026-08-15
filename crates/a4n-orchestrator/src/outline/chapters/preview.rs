use std::collections::HashMap;

use a4n_core::error::AppError;
use a4n_core::models::chapter::part_outline;
use a4n_llm::provider::ChatRequest;
use serde_json::Value;

use crate::outline::chapter_contract::{
    ensure_focus_part_in_context, requested_part_for_chapter, required_part_summary,
    validate_chapter_outline_generation_request, validate_requested_part_context,
};
use crate::outline::context::load_chapter_outline_generation_context;
use crate::outline::{
    OrchestratorContext, OutlineRequestPreviewStep, ready_outline_preview_step,
    reused_outline_preview_step, validate_existing_chapter_outline_for_request,
};
use a4n_prompt::rendering::PromptRenderer;

use super::contracts::ChapterOutlineRequestInput;
use super::requests::build_chapter_outline_request;

impl OrchestratorContext {
    pub async fn preview_regenerate_chapter_outline_request(
        &self,
        project_id: i64,
        outline_id: i64,
        prompt_renderer: &dyn PromptRenderer,
        variables: &HashMap<String, Value>,
    ) -> Result<ChatRequest, AppError> {
        let prepared = self
            .prepare_chapter_outline_regeneration(project_id, outline_id)
            .await?;
        let old = prepared.old;
        let chapter_number = old.chapter_number;
        let context = prepared.context;
        let focus_part = prepared.focus_part;
        build_chapter_outline_request(ChapterOutlineRequestInput {
            project_id,
            context: &context,
            focus_part: focus_part.as_ref(),
            start_chapter: chapter_number,
            count: 1,
            end_chapter: chapter_number,
            prompt_renderer,
            variables,
        })
        .await
    }

    pub async fn preview_generate_chapter_outlines_by_count_requests(
        &self,
        project_id: i64,
        start_chapter: i32,
        count: i32,
        prompt_renderer: &dyn PromptRenderer,
        variables: &HashMap<String, Value>,
    ) -> Result<Vec<OutlineRequestPreviewStep>, AppError> {
        self.preview_chapter_outline_sequence_requests(
            project_id,
            start_chapter,
            count,
            None,
            prompt_renderer,
            variables,
        )
        .await
    }

    pub async fn preview_generate_part_chapter_outline_requests(
        &self,
        project_id: i64,
        part_id: i64,
        prompt_renderer: &dyn PromptRenderer,
        variables: &HashMap<String, Value>,
    ) -> Result<Vec<OutlineRequestPreviewStep>, AppError> {
        let part = self.get_part(part_id).await?;
        if part.project_id != project_id {
            return Err(AppError::InvalidArgument(format!(
                "分部大纲 {part_id} 不属于项目 {project_id}"
            )));
        }
        let count = part.end_chapter - part.start_chapter + 1;
        self.preview_chapter_outline_sequence_requests(
            project_id,
            part.start_chapter,
            count,
            Some(&part),
            prompt_renderer,
            variables,
        )
        .await
    }

    async fn preview_chapter_outline_sequence_requests(
        &self,
        project_id: i64,
        start_chapter: i32,
        count: i32,
        focus_part: Option<&part_outline::Model>,
        prompt_renderer: &dyn PromptRenderer,
        variables: &HashMap<String, Value>,
    ) -> Result<Vec<OutlineRequestPreviewStep>, AppError> {
        let end_chapter = validate_chapter_outline_generation_request(start_chapter, count)?;
        if let Some(part) = focus_part {
            required_part_summary(part)?;
            if start_chapter < part.start_chapter || end_chapter > part.end_chapter {
                return Err(AppError::InvalidArgument(format!(
                    "章节大纲生成范围 {start_chapter}..={end_chapter} 超出分部范围 {}..={}",
                    part.start_chapter, part.end_chapter
                )));
            }
        }

        let context =
            load_chapter_outline_generation_context(self.database.connection(), project_id).await?;
        validate_requested_part_context(&context.parts, start_chapter, end_chapter)?;
        ensure_focus_part_in_context(focus_part, &context.parts)?;

        let mut steps = Vec::new();
        for chapter_number in start_chapter..=end_chapter {
            let expected_part_id =
                requested_part_for_chapter(&context.parts, chapter_number)?.map(|part| part.id);
            let title = format!("第 {chapter_number} 章章纲");
            if let Some(existing) = self
                .existing_chapter_outline_for_number(project_id, chapter_number)
                .await?
            {
                validate_existing_chapter_outline_for_request(
                    &existing,
                    chapter_number,
                    expected_part_id,
                )?;
                steps.push(reused_outline_preview_step(
                    format!("chapter_{chapter_number}"),
                    title,
                    "outline",
                    "已有章节大纲，真实生成会保留并跳过该请求。",
                ));
                continue;
            }

            let request = build_chapter_outline_request(ChapterOutlineRequestInput {
                project_id,
                context: &context,
                focus_part,
                start_chapter: chapter_number,
                count: 1,
                end_chapter: chapter_number,
                prompt_renderer,
                variables,
            })
            .await?;
            steps.push(ready_outline_preview_step(
                format!("chapter_{chapter_number}"),
                title,
                "outline",
                request,
            ));
            break;
        }

        Ok(steps)
    }
}
