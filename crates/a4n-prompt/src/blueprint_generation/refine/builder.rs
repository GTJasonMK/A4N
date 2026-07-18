use super::*;

pub fn build_blueprint_refine_request(
    project_id: i64,
    system_prompt: String,
    user_prompt: String,
    blueprint_context: &Value,
) -> Result<ChatRequest, AppError> {
    let dynamic_user_content_sha256 = source_content_sha256(&user_prompt);
    let manifest = build_blueprint_refine_context_manifest(blueprint_context, user_prompt)?;
    Ok(context_aware_system_user_request(
        system_prompt,
        manifest,
        Some(0.6),
        None,
        Some(llm_token_profiles::BLUEPRINT.into()),
    )?
    .with_json_response_format()
    .with_trace_context(LlmTraceContext {
        project_id: Some(project_id),
        root_project_id: None,
        workflow: "blueprint_refine".to_string(),
        step_key: "blueprint_refine".to_string(),
        generation_run_id: None,
        workflow_run_id: None,
        call_key: Some("blueprint_refine".to_string()),
        input_fingerprint_sha256: Some(source_content_sha256(&serde_json::to_string(&json!({
            "blueprint_context": blueprint_context,
            "dynamic_user_content_sha256": dynamic_user_content_sha256,
        }))?)),
        context_plan_json: None,
        context_pack_refs_json: None,
    }))
}
