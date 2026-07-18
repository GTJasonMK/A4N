use a4n_core::error::AppError;
use a4n_core::models::chapter::chapter_outline;
use a4n_db::generation::{GenerationRunTraceTarget, RecordCompletedGenerationRunInput};
use a4n_llm::{
    provider::ChatRequest,
    trace::{chat_request_messages_json, text_response_summary_json},
};
use serde_json::json;

use crate::{generation::generation_error, outline::OrchestratorContext};
use a4n_llm::trace::context_aware_chat_request_trace_json;

impl OrchestratorContext {
    pub(in crate::outline::chapters) async fn record_chapter_outline_generation_run(
        &self,
        outline: &chapter_outline::Model,
        request: &ChatRequest,
        response_content: &str,
        finish_reason: Option<&str>,
        title: &str,
    ) -> Result<(), AppError> {
        a4n_db::generation::record_completed_generation_run(
            self.db.connection(),
            RecordCompletedGenerationRunInput {
                project_id: outline.project_id,
                workflow: "chapter_outline_generation".to_string(),
                step_key: format!("chapter_{}", outline.chapter_number),
                title: title.to_string(),
                model_kind: "llm".to_string(),
                provider: None,
                model: Some(self.llm.model_name().to_string()),
                prompt_template_names_json: Some(json!(["outline"])),
                prompt_template_hashes_json: None,
                rendered_messages_json: Some(chat_request_messages_json(request)?),
                rendered_prompts_json: None,
                input_variables_json: Some(json!({
                    "chapter_number": outline.chapter_number,
                    "part_id": outline.part_id,
                })),
                source_refs_json: Some(json!({
                    "source_node": if outline.part_id.is_some() {
                        "part_outlines"
                    } else {
                        "blueprint"
                    },
                    "source_kind": if outline.part_id.is_some() {
                        "part_outline"
                    } else {
                        "novel_blueprint"
                    },
                    "part_id": outline.part_id,
                })),
                media_refs_json: None,
                request_json: Some(context_aware_chat_request_trace_json(request)?),
                response_summary_json: Some(text_response_summary_json(
                    response_content,
                    finish_reason,
                    json!({
                        "chapter_number": outline.chapter_number,
                        "title": outline.title,
                    }),
                )),
                targets: vec![GenerationRunTraceTarget {
                    target_kind: "chapter_outline".to_string(),
                    target_table: "chapter_outline".to_string(),
                    target_id: outline.id,
                }],
            },
        )
        .await
        .map_err(generation_error)?;
        Ok(())
    }
}
