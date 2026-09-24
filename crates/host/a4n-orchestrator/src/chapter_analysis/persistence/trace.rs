use a4n_chapter_analysis::{
    CHAPTER_ANALYSIS_STAGE_CHARACTER_STATE, CHAPTER_ANALYSIS_STAGE_FACTS,
    CHAPTER_ANALYSIS_STAGE_FORESHADOWING_EVENTS, CHAPTER_ANALYSIS_STAGE_SUMMARY,
};
use a4n_db_models::models::chapter::chapter;
use a4n_domain::generation::{GenerationRunTraceTarget, RecordCompletedGenerationRunInput};
use a4n_error::AppError;
use a4n_llm_trace::{chat_request_messages_json, context_aware_chat_request_trace_json};
use serde_json::{Value, json};

use super::super::OrchestratorContext;
use super::super::execution::ChapterAnalysisRun;
use super::super::owner::json_value_shape_summary;

impl OrchestratorContext {
    pub(in crate::chapter_analysis) fn chapter_analysis_generation_run_input(
        &self,
        chapter: &chapter::Model,
        run: &ChapterAnalysisRun,
    ) -> Result<RecordCompletedGenerationRunInput, AppError> {
        Ok(RecordCompletedGenerationRunInput {
            project_id: chapter.project_id,
            workflow: "chapter_analysis".to_string(),
            step_key: format!("chapter_{}", chapter.chapter_number),
            title: "生成章节分析".to_string(),
            model_kind: "llm".to_string(),
            provider: None,
            model: Some(self.llm_model_name()),
            prompt_template_names_json: Some(json!([
                CHAPTER_ANALYSIS_STAGE_FACTS,
                CHAPTER_ANALYSIS_STAGE_SUMMARY,
                CHAPTER_ANALYSIS_STAGE_CHARACTER_STATE,
                CHAPTER_ANALYSIS_STAGE_FORESHADOWING_EVENTS,
            ])),
            prompt_template_hashes_json: None,
            rendered_messages_json: Some(Value::Array(
                run.stage_traces
                    .iter()
                    .map(|trace| {
                        Ok(json!({
                            "stage": trace.stage,
                            "messages": chat_request_messages_json(&trace.request)?,
                        }))
                    })
                    .collect::<Result<Vec<_>, AppError>>()?,
            )),
            rendered_prompts_json: None,
            input_variables_json: Some(json!({
                "chapter_number": chapter.chapter_number,
                "title": chapter.title,
                "content_chars": chapter.content.chars().count(),
            })),
            source_refs_json: Some(json!({
                "source_node": "chapters",
                "source_kind": "chapter_content",
                "chapter_id": chapter.id,
                "content_sha256": run.analysis
                    .get("_meta")
                    .and_then(Value::as_object)
                    .and_then(|meta| meta.get("content_sha256"))
                    .cloned(),
            })),
            media_refs_json: None,
            request_json: Some(Value::Array(
                run.stage_traces
                    .iter()
                    .map(|trace| {
                        Ok(json!({
                            "stage": trace.stage,
                            "request": context_aware_chat_request_trace_json(&trace.request)?,
                        }))
                    })
                    .collect::<Result<Vec<_>, AppError>>()?,
            )),
            response_summary_json: Some(json!({
                "chapter_number": chapter.chapter_number,
                "stage_count": run.stage_traces.len(),
                "stage_outputs": run.stage_traces
                    .iter()
                    .map(|trace| json!({
                        "stage": trace.stage,
                        "summary": json_value_shape_summary(&trace.output),
                    }))
                    .collect::<Vec<_>>(),
            })),
            targets: vec![GenerationRunTraceTarget {
                target_kind: "chapter_analysis".to_string(),
                target_table: "chapter_analysis".to_string(),
                target_id: chapter.id,
            }],
        })
    }
}
