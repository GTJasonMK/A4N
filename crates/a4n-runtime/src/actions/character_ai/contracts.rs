pub use a4n_api_contract::character::{
    SyncAllCharacterProfilesInput, SyncAllCharacterProfilesResult, SyncCharacterProfileInput,
    SyncCharacterProfileResult,
};
use a4n_llm::provider::ChatRequest;

#[derive(Debug, Clone, Copy)]
pub struct CharacterAiTaskScope {
    pub project_id: i64,
    pub total: u32,
}

#[derive(Debug, Clone)]
pub struct CharacterAiPreviewStep {
    pub key: String,
    pub title: String,
    pub prompt_name: Option<String>,
    pub state: String,
    pub note: Option<String>,
    pub request: Option<ChatRequest>,
}

impl CharacterAiPreviewStep {
    pub(crate) fn llm(
        key: impl Into<String>,
        title: impl Into<String>,
        prompt_name: &str,
        state: impl Into<String>,
        note: Option<String>,
        request: ChatRequest,
    ) -> Self {
        Self {
            key: key.into(),
            title: title.into(),
            prompt_name: Some(prompt_name.to_string()),
            state: state.into(),
            note,
            request: Some(request),
        }
    }

    pub(crate) fn status(
        key: impl Into<String>,
        title: impl Into<String>,
        prompt_name: &str,
        state: impl Into<String>,
        note: Option<String>,
    ) -> Self {
        Self {
            key: key.into(),
            title: title.into(),
            prompt_name: Some(prompt_name.to_string()),
            state: state.into(),
            note,
            request: None,
        }
    }
}
