use a4n_llm_provider_contract::provider::ChatRequest;

use super::super::contracts::MangaPipelineRequestPreviewStep;

impl MangaPipelineRequestPreviewStep {
    pub(in crate::manga::pipeline_preview) fn ready(
        key: &str,
        title: &str,
        prompt_name: &str,
        note: Option<String>,
        request: ChatRequest,
    ) -> Self {
        Self {
            key: key.to_string(),
            title: title.to_string(),
            prompt_name: Some(prompt_name.to_string()),
            state: "ready".to_string(),
            note,
            request: Some(request),
        }
    }

    pub(in crate::manga::pipeline_preview) fn reused(
        key: &str,
        title: &str,
        prompt_name: &str,
        note: String,
    ) -> Self {
        Self {
            key: key.to_string(),
            title: title.to_string(),
            prompt_name: Some(prompt_name.to_string()),
            state: "reused".to_string(),
            note: Some(note),
            request: None,
        }
    }

    pub(in crate::manga::pipeline_preview) fn reused_checkpoint(
        key: &str,
        title: &str,
        prompt_name: &str,
    ) -> Self {
        Self::reused(
            key,
            title,
            prompt_name,
            "已有匹配的调用级 checkpoint，本次不会重新发送该请求".to_string(),
        )
    }
}
