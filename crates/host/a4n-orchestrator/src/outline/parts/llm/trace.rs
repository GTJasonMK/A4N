use a4n_db::DbTransaction as DatabaseTransaction;
use a4n_domain::chapter::PartOutlineRecord;
use a4n_domain::generation::{GenerationRunTraceTarget, RecordCompletedGenerationRunInput};
use a4n_error::AppError;
use a4n_llm_trace::{chat_request_messages_json, text_response_summary_json};
use serde_json::json;

use crate::{generation::generation_error, outline::OrchestratorContext};
use a4n_llm_trace::context_aware_chat_request_trace_json;

use super::super::contracts::GeneratedPartOutline;

impl OrchestratorContext {
    pub(in crate::outline::parts) async fn record_part_outline_generation_run(
        &self,
        db: &DatabaseTransaction,
        part: &PartOutlineRecord,
        generated: &GeneratedPartOutline,
        title: &str,
    ) -> Result<(), AppError> {
        a4n_db::generation::record_completed_generation_run_in_transaction(
            db,
            RecordCompletedGenerationRunInput {
                project_id: part.project_id,
                workflow: "part_outline_generation".to_string(),
                step_key: format!("part_{}", part.part_number),
                title: title.to_string(),
                model_kind: "llm".to_string(),
                provider: None,
                model: Some(self.llm_model_name()),
                prompt_template_names_json: Some(json!(["part_outline_single"])),
                prompt_template_hashes_json: None,
                rendered_messages_json: Some(chat_request_messages_json(&generated.request)?),
                rendered_prompts_json: None,
                input_variables_json: Some(json!({
                    "part_number": part.part_number,
                    "start_chapter": part.start_chapter,
                    "end_chapter": part.end_chapter,
                })),
                source_refs_json: Some(json!({
                    "source_node": "blueprint",
                    "source_kind": "novel_blueprint",
                    "previous_part_context": true,
                })),
                media_refs_json: None,
                request_json: Some(context_aware_chat_request_trace_json(&generated.request)?),
                response_summary_json: Some(text_response_summary_json(
                    &generated.response_content,
                    generated.finish_reason.as_deref(),
                    json!({
                        "part_number": part.part_number,
                        "title": part.title,
                    }),
                )),
                targets: vec![GenerationRunTraceTarget {
                    target_kind: "part_outline".to_string(),
                    target_table: "part_outline".to_string(),
                    target_id: part.id,
                }],
            },
        )
        .await
        .map_err(generation_error)?;
        Ok(())
    }
}
