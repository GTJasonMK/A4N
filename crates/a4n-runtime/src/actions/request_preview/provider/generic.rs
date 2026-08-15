use serde::Serialize;
use serde_json::json;

use super::super::{
    contracts::ModelRequestPreviewStep, sanitize::sanitize_preview_value,
    status::prompt_from_request,
};

pub(crate) fn provider_request_step<T: Serialize>(
    key: impl Into<String>,
    title: impl Into<String>,
    kind: impl Into<String>,
    prompt_name: impl Into<Option<String>>,
    state: impl Into<String>,
    note: impl Into<Option<String>>,
    request: &T,
) -> Result<ModelRequestPreviewStep, String> {
    let request = sanitize_preview_value(serde_json::to_value(request).map_err(|e| e.to_string())?);
    let request_text = request.to_string();
    let total_chars = request_text.chars().count();
    let prompt = prompt_from_request(&request);
    let prompt_chars = prompt.as_deref().map(str::chars).map(Iterator::count);
    let prompt_bytes = prompt.as_deref().map(str::len);
    Ok(ModelRequestPreviewStep {
        key: key.into(),
        title: title.into(),
        kind: kind.into(),
        prompt_name: prompt_name.into(),
        state: state.into(),
        note: note.into(),
        system_prompt: None,
        user_prompt: prompt,
        messages: None,
        request: Some(request),
        context_manifest: None,
        context_plan: None,
        context_pack_refs: None,
        total_chars,
        char_counts: Some(json!({
            "prompt_chars": prompt_chars,
            "prompt_bytes": prompt_bytes,
            "request_chars": total_chars,
            "request_bytes": request_text.len()
        })),
    })
}
