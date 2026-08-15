use a4n_llm::provider::ChatRequest;

use super::OutlineRequestPreviewStep;

pub fn ready_outline_preview_step(
    key: impl Into<String>,
    title: impl Into<String>,
    prompt_name: impl Into<String>,
    request: ChatRequest,
) -> OutlineRequestPreviewStep {
    OutlineRequestPreviewStep {
        key: key.into(),
        title: title.into(),
        prompt_name: prompt_name.into(),
        state: "ready".to_string(),
        note: None,
        request: Some(request),
    }
}

pub fn reused_outline_preview_step(
    key: impl Into<String>,
    title: impl Into<String>,
    prompt_name: impl Into<String>,
    note: impl Into<String>,
) -> OutlineRequestPreviewStep {
    OutlineRequestPreviewStep {
        key: key.into(),
        title: title.into(),
        prompt_name: prompt_name.into(),
        state: "reused".to_string(),
        note: Some(note.into()),
        request: None,
    }
}
