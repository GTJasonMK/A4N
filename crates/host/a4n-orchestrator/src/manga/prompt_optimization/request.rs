use std::collections::HashMap;

use a4n_domain::{manga::MangaPagePromptRecord, source_text_hash};
use a4n_error::AppError;
use a4n_llm_provider_contract::provider::{ChatRequest, LlmTraceContext};
use a4n_prompt::rendering::PromptRenderer;
use serde_json::json;

pub(super) const MANGA_PROMPT_OPTIMIZATION_PROMPT_NAME: &str = "manga_prompt_optimization";
pub(super) const MANGA_PROMPT_OPTIMIZATION_USER_PROMPT_NAME: &str =
    "manga_prompt_optimization_user";

pub(super) fn manga_prompt_optimization_call_key(prompt_id: i64) -> String {
    format!("optimize_manga_prompt:prompt:{prompt_id}")
}

pub(super) async fn build_manga_prompt_optimization_request(
    prompt_renderer: &dyn PromptRenderer,
    prompt: &MangaPagePromptRecord,
    instruction: &str,
) -> Result<ChatRequest, AppError> {
    let system_prompt = prompt_renderer
        .render_prompt_for_call(
            "manga_records::optimize_manga_prompt",
            MANGA_PROMPT_OPTIMIZATION_PROMPT_NAME,
            &HashMap::new(),
            Some(prompt.project_id),
        )
        .await?;
    let payload = json!({
        "current_prompt": {
            "prompt_id": prompt.id,
            "page_number": prompt.page_number,
            "dialogue_text": prompt.dialogue_text,
            "image_prompt": prompt.image_prompt,
        },
        "instruction": instruction,
        "output_contract": {
            "format": "json_object",
            "required_fields": ["image_prompt"],
            "field_rules": {"image_prompt": "优化后的整页漫画绘图提示词，不能为空"},
        },
    });
    let mut user_variables = HashMap::new();
    user_variables.insert(
        "payload".to_string(),
        json!(serde_json::to_string_pretty(&payload)?),
    );
    let user_prompt = prompt_renderer
        .render_prompt_for_call(
            "manga_records::optimize_manga_prompt",
            MANGA_PROMPT_OPTIMIZATION_USER_PROMPT_NAME,
            &user_variables,
            Some(prompt.project_id),
        )
        .await?;
    let mut request = ChatRequest::system_user(system_prompt, user_prompt, Some(0.25), None)
        .with_prompt_assembly_key("manga_records::optimize_manga_prompt")
        .with_json_schema_response_format(
            MANGA_PROMPT_OPTIMIZATION_PROMPT_NAME,
            manga_prompt_optimization_output_schema(),
            false,
        )?;
    let input_fingerprint_sha256 =
        source_text_hash(&serde_json::to_string(&request).map_err(AppError::Serialization)?);
    request = request.with_trace_context(LlmTraceContext {
        project_id: Some(prompt.project_id),
        root_project_id: None,
        workflow: "manga_prompt_optimization".to_string(),
        step_key: "optimize_manga_prompt".to_string(),
        generation_run_id: None,
        workflow_run_id: None,
        call_key: Some(manga_prompt_optimization_call_key(prompt.id)),
        input_fingerprint_sha256: Some(input_fingerprint_sha256),
        context_plan_json: None,
        context_pack_refs_json: None,
    });
    Ok(request)
}

pub(super) fn manga_prompt_optimization_output_schema() -> serde_json::Value {
    a4n_domain::json_contract::typed_output_schema::<super::execution::OptimizedMangaPromptFields>()
}

#[cfg(test)]
#[test]
fn model_contract_matrix() {
    a4n_test_support::model_contract::emit_schema(
        MANGA_PROMPT_OPTIMIZATION_PROMPT_NAME,
        manga_prompt_optimization_output_schema(),
    );
}
