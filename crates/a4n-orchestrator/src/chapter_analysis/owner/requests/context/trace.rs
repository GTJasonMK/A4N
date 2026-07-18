use a4n_llm::provider::LlmTraceContext;

pub(super) fn chapter_analysis_trace_context(
    project_id: i64,
    prompt_name: &str,
) -> LlmTraceContext {
    LlmTraceContext {
        project_id: Some(project_id),
        root_project_id: None,
        workflow: "chapter_analysis".to_string(),
        step_key: prompt_name.to_string(),
        generation_run_id: None,
        workflow_run_id: None,
        call_key: None,
        input_fingerprint_sha256: None,
        context_plan_json: None,
        context_pack_refs_json: None,
    }
}
