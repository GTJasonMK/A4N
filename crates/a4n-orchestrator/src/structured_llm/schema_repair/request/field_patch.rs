use a4n_core::error::AppError;
use a4n_llm::provider::{ChatMessage, ChatRequest};
use serde_json::json;

use super::{
    super::{MAX_SCHEMA_PATCH_TOKENS, contract::StructuredSchemaReport},
    manifest::{RepairOutputContract, rewrite_manifest_repair_request},
    trace::{refresh_trace_fingerprint, suffix_trace},
};

pub(in crate::structured_llm::schema_repair) fn build_schema_field_patch_request(
    mut request: ChatRequest,
    repair_prompt: &str,
    report: &StructuredSchemaReport,
) -> Result<ChatRequest, AppError> {
    request.temperature = Some(0.0);
    request.max_tokens = Some(MAX_SCHEMA_PATCH_TOKENS);
    request.max_tokens_profile = None;
    request.response_format = Some(a4n_llm::provider::ResponseFormat::json_object());
    request.tools = None;
    request.tool_choice = None;
    request.stop_sequences = None;
    let task = json!({
        "schema_version": "structured_json_field_patch_request_v1",
        "target_schema_sha256": report.schema_sha256,
        "diagnostics": report.diagnostics,
    });
    let task = format!(
        "本次字段补丁任务：\n{}",
        serde_json::to_string_pretty(&task)?
    );
    if !rewrite_manifest_repair_request(
        &mut request,
        &task,
        RepairOutputContract::Replace(repair_prompt),
    )? {
        request
            .messages
            .push(ChatMessage::user(format!("{repair_prompt}\n\n{task}")));
    }
    suffix_trace(&mut request, "schema_patch");
    refresh_trace_fingerprint(&mut request)?;
    Ok(request)
}
