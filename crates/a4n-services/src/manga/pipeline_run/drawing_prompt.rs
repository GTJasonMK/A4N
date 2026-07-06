use std::collections::HashMap;

use a4n_core::error::AppError;
use a4n_engine::llm::provider::LlmProvider;
use a4n_engine::prompt::engine::PromptEngine;
use serde_json::{Map, Value, json};

use crate::chapter::source_spans::SourceSpan;

use super::super::checkpoint::{checkpoint_insert, checkpoint_insert_trace};
use super::super::pipeline_json::{
    page_drawing_prompt_from_response, page_plan_page_number, page_plan_pages, parse_json_object,
};
use super::super::pipeline_persist::PageMangaPromptGenerationTrace;
use super::super::pipeline_prompt::{
    MANGA_PREVIOUS_OUTPUT_EXTRACTION_SUMMARY, MANGA_PREVIOUS_OUTPUT_PAGE_PLANNING,
    MangaContextManifestInput, MangaContextPreviousOutput, MangaPromptGenerationTrace,
    MangaRenderTraceInput, build_manga_context_manifest, manga_page_drawing_prompt_user_message,
    render_and_chat_trace,
};
use super::super::pipeline_status::{MangaPipelineCheckpointInput, MangaPipelineStatusInput};
use super::super::{MangaService, PIPELINE_TOTAL_STEPS};

pub(super) struct DrawingPromptStepInput<'a> {
    pub(super) chapter_id: i64,
    pub(super) project_id: i64,
    pub(super) llm: &'a dyn LlmProvider,
    pub(super) prompt_engine: &'a dyn PromptEngine,
    pub(super) vars: &'a HashMap<String, Value>,
    pub(super) task_id: Option<&'a str>,
    pub(super) workflow_run_id: Option<i64>,
    pub(super) source_metadata: &'a Value,
    pub(super) reference_assets_metadata: &'a Value,
    pub(super) prompt_fingerprint: &'a Value,
    pub(super) checkpoint: &'a mut Map<String, Value>,
    pub(super) chapter_content: &'a str,
    pub(super) source_spans: &'a [SourceSpan],
    pub(super) reference_assets: &'a Value,
    pub(super) plan_json: &'a Value,
    pub(super) extraction_summary: &'a Value,
    pub(super) restore_from_checkpoint: bool,
}

pub(super) struct DrawingPromptStepOutput {
    pub(super) json: Value,
    pub(super) page_traces: Vec<PageMangaPromptGenerationTrace>,
}

impl MangaService {
    pub(super) async fn run_page_drawing_prompt_step(
        &self,
        input: DrawingPromptStepInput<'_>,
    ) -> Result<DrawingPromptStepOutput, AppError> {
        let DrawingPromptStepInput {
            chapter_id,
            project_id,
            llm,
            prompt_engine,
            vars,
            task_id,
            workflow_run_id,
            source_metadata,
            reference_assets_metadata,
            prompt_fingerprint,
            checkpoint,
            chapter_content,
            source_spans,
            reference_assets,
            plan_json,
            extraction_summary,
            restore_from_checkpoint,
        } = input;
        let pages = page_plan_pages(plan_json, "manga_page_planning")?;
        let mut drawing_pages = Vec::with_capacity(pages.len());
        let mut page_traces = Vec::new();
        for (index, page_plan) in pages.iter().enumerate() {
            let page_output = self
                .run_single_page_drawing_prompt(SinglePageDrawingPromptInput {
                    chapter_id,
                    project_id,
                    llm,
                    prompt_engine,
                    vars,
                    task_id,
                    workflow_run_id,
                    source_metadata,
                    reference_assets_metadata,
                    prompt_fingerprint,
                    checkpoint,
                    chapter_content,
                    source_spans,
                    reference_assets,
                    plan_json,
                    page_plan,
                    page_index: index,
                    total_pages: pages.len(),
                    extraction_summary,
                    restore_from_checkpoint,
                })
                .await?;
            page_traces.push(PageMangaPromptGenerationTrace {
                page_number: page_output.page_number,
                trace: page_output.trace,
            });
            drawing_pages.push(page_output.prompt);
        }
        checkpoint_insert(checkpoint, "dp_page_traces", json!(page_traces));
        let json = json!({ "pages": drawing_pages });

        Ok(DrawingPromptStepOutput { json, page_traces })
    }

    async fn run_single_page_drawing_prompt(
        &self,
        input: SinglePageDrawingPromptInput<'_>,
    ) -> Result<SinglePageDrawingPromptOutput, AppError> {
        let SinglePageDrawingPromptInput {
            chapter_id,
            project_id,
            llm,
            prompt_engine,
            vars,
            task_id,
            workflow_run_id,
            source_metadata,
            reference_assets_metadata,
            prompt_fingerprint,
            checkpoint,
            chapter_content,
            source_spans,
            reference_assets,
            plan_json,
            page_plan,
            page_index,
            total_pages,
            extraction_summary,
            restore_from_checkpoint,
        } = input;
        let page_number = page_plan_page_number(
            page_plan,
            &format!("manga_page_planning.pages[{page_index}]"),
        )?;
        let page_out_key = format!("dp_page_{page_number}_out");
        let page_trace_key = format!("dp_page_{page_number}_trace");
        let page_json_key = format!("dp_page_{page_number}_json");
        let page_context = format!("manga_page_drawing_prompt.page_{page_number}");
        let dp_user =
            manga_page_drawing_prompt_user_message(page_number, page_plan, extraction_summary)?;
        let plan_context = json!({ "page_plan": page_plan, "plan": plan_json });
        let dp_manifest = build_manga_context_manifest(MangaContextManifestInput {
            prompt_engine,
            project_id,
            variables: vars,
            chapter_content,
            source_spans,
            prompt_names: &["manga_page_planning", "manga_page_drawing_prompt"],
            previous_outputs: &[
                MangaContextPreviousOutput {
                    pack_kind: MANGA_PREVIOUS_OUTPUT_PAGE_PLANNING,
                    prompt_name: "manga_page_planning",
                    parsed_output: &plan_context,
                    raw_output: None,
                    placeholder: false,
                },
                MangaContextPreviousOutput {
                    pack_kind: MANGA_PREVIOUS_OUTPUT_EXTRACTION_SUMMARY,
                    prompt_name: "manga_extraction_summary",
                    parsed_output: extraction_summary,
                    raw_output: None,
                    placeholder: false,
                },
            ],
            reference_assets: Some(reference_assets),
            dynamic_user_content: dp_user,
        })?;
        let trace = render_and_chat_trace(MangaRenderTraceInput {
            db: self.db.clone(),
            prompt_engine,
            llm,
            prompt_name: "manga_page_drawing_prompt",
            project_id,
            workflow_run_id,
            call_key: format!(
                "manga_pipeline:{chapter_id}:manga_page_drawing_prompt:page:{page_number}"
            ),
            call_order: 800 + page_index as i32,
            restore_from_checkpoint,
            variables: vars,
            manifest: dp_manifest,
            temperature: 0.5,
        })
        .await?;
        self.report_pipeline_llm_usage(
            input.chapter_id,
            input.project_id,
            "drawing_prompt",
            8,
            input.task_id,
            &trace,
        )
        .await?;
        let page_out = trace.response_text.clone();
        let page_json = parse_json_object(&page_out, &page_context)?;
        let page_prompt =
            page_drawing_prompt_from_response(&page_json, page_number, &page_context)?;
        self.save_or_validate_manga_llm_checkpoint(&trace, &page_json)
            .await?;
        checkpoint_insert(checkpoint, &page_out_key, json!(page_out));
        checkpoint_insert_trace(checkpoint, &page_trace_key, &trace)?;
        checkpoint_insert(checkpoint, &page_json_key, page_json);
        let message = format!(
            "已完成第 {page_number} 页整页提示词 ({}/{total_pages})",
            page_index + 1
        );
        let checkpoint_data = Value::Object(checkpoint.clone());
        self.save_pipeline_checkpoint(MangaPipelineCheckpointInput {
            status: MangaPipelineStatusInput {
                chapter_id,
                project_id,
                pipeline_status: "running",
                stage: "drawing_prompt",
                current: 8,
                total: PIPELINE_TOTAL_STEPS,
                message: &message,
                task_id,
                allow_cancel_override: false,
            },
            source_metadata,
            reference_assets_metadata,
            prompt_fingerprint,
            checkpoint_data: &checkpoint_data,
        })
        .await?;
        self.ensure_pipeline_not_cancelled(project_id, chapter_id)
            .await?;

        Ok(SinglePageDrawingPromptOutput {
            page_number,
            prompt: page_prompt,
            trace,
        })
    }
}

struct SinglePageDrawingPromptOutput {
    page_number: i32,
    prompt: Value,
    trace: MangaPromptGenerationTrace,
}

struct SinglePageDrawingPromptInput<'a> {
    chapter_id: i64,
    project_id: i64,
    llm: &'a dyn LlmProvider,
    prompt_engine: &'a dyn PromptEngine,
    vars: &'a HashMap<String, Value>,
    task_id: Option<&'a str>,
    workflow_run_id: Option<i64>,
    source_metadata: &'a Value,
    reference_assets_metadata: &'a Value,
    prompt_fingerprint: &'a Value,
    checkpoint: &'a mut Map<String, Value>,
    chapter_content: &'a str,
    source_spans: &'a [SourceSpan],
    reference_assets: &'a Value,
    plan_json: &'a Value,
    page_plan: &'a Value,
    page_index: usize,
    total_pages: usize,
    extraction_summary: &'a Value,
    restore_from_checkpoint: bool,
}
