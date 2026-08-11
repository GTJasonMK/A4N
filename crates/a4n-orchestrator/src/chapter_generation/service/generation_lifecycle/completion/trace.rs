use super::ChapterGenerationTrace;
use a4n_core::{
    error::AppError,
    models::chapter::{chapter, chapter_outline},
};
use a4n_db::generation::{GenerationRunTraceTarget, RecordCompletedGenerationRunInput};
use a4n_llm::trace::{chat_request_messages_json, text_response_summary_json};

use crate::chapter_generation::service::ChapterGenerationService;
use crate::generation::generation_error;
use a4n_llm::trace::context_aware_chat_request_trace_json;
use sea_orm::DatabaseTransaction;

impl ChapterGenerationService {
    pub(in crate::chapter_generation) async fn record_chapter_generation_trace(
        &self,
        chapter: &chapter::Model,
        source_outline: Option<&chapter_outline::Model>,
        chapter_outline: &str,
        content: &str,
        trace: ChapterGenerationTrace,
        db: &DatabaseTransaction,
    ) -> Result<(), AppError> {
        a4n_db::generation::record_completed_generation_run_in_transaction(
            db,
            RecordCompletedGenerationRunInput {
                project_id: chapter.project_id,
                workflow: "chapter_generation".to_string(),
                step_key: format!("chapter_{}", chapter.chapter_number),
                title: "生成章节正文".to_string(),
                model_kind: "llm".to_string(),
                provider: None,
                model: Some(trace.model_name),
                prompt_template_names_json: Some(serde_json::json!(["writing"])),
                prompt_template_hashes_json: None,
                rendered_messages_json: Some(chat_request_messages_json(&trace.request)?),
                rendered_prompts_json: None,
                input_variables_json: Some(serde_json::json!({
                    "chapter_number": chapter.chapter_number,
                    "chapter_outline": chapter_outline,
                })),
                source_refs_json: source_outline.map(|outline| {
                    serde_json::json!({
                        "source_node": "chapter_outlines",
                        "source_kind": "chapter_outline",
                        "outline_id": outline.id,
                        "updated_at": outline.updated_at,
                    })
                }),
                media_refs_json: None,
                request_json: Some(context_aware_chat_request_trace_json(&trace.request)?),
                response_summary_json: Some(text_response_summary_json(
                    content,
                    trace.finish_reason.as_deref(),
                    serde_json::json!({
                        "title": chapter.title,
                        "word_count": chapter.word_count,
                    }),
                )),
                targets: vec![GenerationRunTraceTarget {
                    target_kind: "chapter_content".to_string(),
                    target_table: "chapter_content".to_string(),
                    target_id: chapter.id,
                }],
            },
        )
        .await
        .map_err(generation_error)
        .map(|_| ())
    }
}
