use serde::Serialize;
use serde_json::json;

use super::super::{
    contracts::ModelRequestPreviewStep, sanitize::sanitize_preview_value,
    status::prompt_from_request,
};

pub(crate) async fn attach_prompt_assembly_metadata(
    state: &crate::AppState,
    project_id: Option<i64>,
    call_key: &str,
    prompt_name: &str,
    step: &mut ModelRequestPreviewStep,
) -> Result<(), String> {
    let context = crate::actions::prompt_records::prompt_context(state);
    let assembly = state
        .prompt_read_application()
        .resolve_assembly_record(&context, call_key, prompt_name, project_id)
        .await
        .map_err(|error| error.to_string())?;
    step.prompt_name = Some(prompt_name.to_string());
    step.prompt_assembly_key = Some(call_key.to_string());
    step.prompt_assembly_source = Some(
        match assembly.source {
            a4n_api_contract::prompt::PromptAssemblySource::SystemDefault => "system_default",
            a4n_api_contract::prompt::PromptAssemblySource::Global => "global",
            a4n_api_contract::prompt::PromptAssemblySource::Project => "project",
        }
        .to_string(),
    );
    step.prompt_assembly_blocks = Some(
        assembly
            .config
            .blocks
            .into_iter()
            .filter(|block| block.enabled)
            .map(|block| match block.kind {
                a4n_api_contract::prompt::PromptAssemblyBlockKind::Template => {
                    format!("模板 {}", block.value)
                }
                a4n_api_contract::prompt::PromptAssemblyBlockKind::Variable => {
                    format!("变量 {}", block.value)
                }
                a4n_api_contract::prompt::PromptAssemblyBlockKind::Custom => "自定义模块".into(),
            })
            .collect(),
    );
    Ok(())
}

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
        prompt_assembly_source: None,
        prompt_assembly_blocks: None,
        prompt_assembly_key: None,
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
