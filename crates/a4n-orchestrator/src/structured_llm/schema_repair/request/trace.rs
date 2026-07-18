use a4n_core::error::AppError;
use a4n_llm::provider::ChatRequest;

pub(super) fn suffix_trace(request: &mut ChatRequest, suffix: &str) {
    if let Some(trace) = request.trace_context.as_mut() {
        trace.step_key = format!("{}:{suffix}", trace.step_key);
        trace.call_key = trace
            .call_key
            .as_ref()
            .map(|call_key| format!("{call_key}:{suffix}"));
    }
}

pub(super) fn refresh_trace_fingerprint(request: &mut ChatRequest) -> Result<(), AppError> {
    let fingerprint_json = serde_json::to_value(&*request)?;
    if let Some(trace) = request.trace_context.as_mut() {
        trace.input_fingerprint_sha256 =
            Some(a4n_db::generation::generation_checkpoint_fingerprint_sha256(&fingerprint_json)?);
    }
    Ok(())
}
