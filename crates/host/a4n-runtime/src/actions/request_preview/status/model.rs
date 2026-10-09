use a4n_llm_provider_contract::provider::ChatRequest;

use super::super::contracts::ModelRequestPreviewStep;

pub(in crate::actions::request_preview) fn status_step(
    key: impl Into<String>,
    title: impl Into<String>,
    kind: impl Into<String>,
    prompt_name: impl Into<Option<String>>,
    state: impl Into<String>,
    note: impl Into<Option<String>>,
) -> ModelRequestPreviewStep {
    ModelRequestPreviewStep {
        key: key.into(),
        title: title.into(),
        kind: kind.into(),
        prompt_name: prompt_name.into(),
        prompt_assembly_source: None,
        prompt_assembly_blocks: None,
        prompt_assembly_key: None,
        state: state.into(),
        note: note.into(),
        system_prompt: None,
        user_prompt: None,
        messages: None,
        request: None,
        context_manifest: None,
        context_plan: None,
        context_pack_refs: None,
        total_chars: 0,
        char_counts: None,
    }
}

pub(crate) struct ChatStatusPreviewStep {
    pub(crate) key: String,
    pub(crate) title: String,
    pub(crate) kind: Option<String>,
    pub(crate) prompt_name: Option<String>,
    pub(crate) state: String,
    pub(crate) note: Option<String>,
    pub(crate) request: Option<ChatRequest>,
}

impl ChatStatusPreviewStep {
    pub(crate) fn llm(
        key: impl Into<String>,
        title: impl Into<String>,
        prompt_name: impl Into<Option<String>>,
        state: impl Into<String>,
        note: impl Into<Option<String>>,
        request: Option<ChatRequest>,
    ) -> Self {
        Self::new(key, title, None, prompt_name, state, note, request)
    }

    pub(crate) fn new(
        key: impl Into<String>,
        title: impl Into<String>,
        kind: impl Into<Option<String>>,
        prompt_name: impl Into<Option<String>>,
        state: impl Into<String>,
        note: impl Into<Option<String>>,
        request: Option<ChatRequest>,
    ) -> Self {
        Self {
            key: key.into(),
            title: title.into(),
            kind: kind.into(),
            prompt_name: prompt_name.into(),
            state: state.into(),
            note: note.into(),
            request,
        }
    }
}
