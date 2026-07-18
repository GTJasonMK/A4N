use a4n_core::error::AppError;
use serde_json::{Value, json};

use a4n_llm::trace::context_aware_chat_request_trace_json;

pub fn chapter_generation_call_fingerprint_json(
    project_id: i64,
    chapter_number: i32,
    request: &a4n_llm::provider::ChatRequest,
) -> Result<Value, AppError> {
    Ok(json!({
        "workflow": "chapter_generation",
        "project_id": project_id,
        "chapter_number": chapter_number,
        "request": context_aware_chat_request_trace_json(request)?,
    }))
}
