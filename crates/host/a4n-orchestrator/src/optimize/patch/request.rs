use a4n_error::AppError;
use a4n_llm_context::LlmContextManifest;
use a4n_llm_provider_contract::provider::{ChatRequest, LlmTraceContext};
use serde_json::json;

use a4n_domain::source_text_hash as source_content_sha256;
use a4n_llm_context::{
    COMPRESSION_FULL_RAW, LlmContextLedger, LlmContextLedgerPackDraft,
    context_aware_system_user_request,
};
use a4n_llm_trace::context_aware_chat_request_trace_json;

pub(in crate::optimize) fn build_content_optimization_patch_request(
    project_id: i64,
    session_id: i64,
    system_prompt: String,
    user_message: String,
    chapter_content: &str,
    project_context: &serde_json::Value,
) -> Result<ChatRequest, AppError> {
    let manifest = build_content_optimization_patch_context_manifest(
        chapter_content,
        project_context,
        user_message,
    )?;
    Ok(
        context_aware_system_user_request(system_prompt, manifest, Some(0.2), None)?
            .with_json_response_format()
            .with_trace_context(LlmTraceContext {
                project_id: Some(project_id),
                root_project_id: None,
                workflow: "content_optimization_patch".to_string(),
                step_key: format!("session_{session_id}_patch"),
                generation_run_id: None,
                workflow_run_id: None,
                call_key: Some(format!("session_{session_id}_patch")),
                input_fingerprint_sha256: None,
                context_plan_json: None,
                context_pack_refs_json: None,
            }),
    )
}

pub(in crate::optimize) fn optimize_llm_call_input_fingerprint_json(
    request: &ChatRequest,
) -> Result<serde_json::Value, AppError> {
    let mut value = context_aware_chat_request_trace_json(request)?;
    if let Some(trace) = value
        .get_mut("trace_context")
        .and_then(serde_json::Value::as_object_mut)
    {
        trace.remove("input_fingerprint_sha256");
    }
    Ok(value)
}

fn build_content_optimization_patch_context_manifest(
    chapter_content: &str,
    project_context: &serde_json::Value,
    dynamic_user_content: String,
) -> Result<LlmContextManifest, AppError> {
    let chapter_content = chapter_content.trim();
    if chapter_content.is_empty() {
        return Err(AppError::InvalidArgument(
            "内容优化补丁缺少章节正文".to_string(),
        ));
    }
    let mut ledger = LlmContextLedger::new();
    let content_hash = source_content_sha256(chapter_content);
    ledger.add_pack(a4n_llm_context::LlmContextLedgerPackInput {
        scope_kind: ("chapter").into(),
        pack_kind: ("optimization_patch_chapter_content").into(),
        source_refs_json: Some(json!({
            "source": "chapter_content",
            "content_sha256": content_hash,
        })),
        config_identity_json: json!({
            "ledger_version": "optimization_patch_context_v1",
            "kind": "chapter_content",
        }),
        required: true,
        stable_prefix: true,
        variants: vec![LlmContextLedgerPackDraft::new(
            COMPRESSION_FULL_RAW,
            json!({"kind": "chapter_content", "content": chapter_content}),
        )],
    })?;

    let mut project_context = project_context.clone();
    if let Some(chapter) = project_context
        .get_mut("current_chapter")
        .and_then(serde_json::Value::as_object_mut)
    {
        // 补丁定位使用前面的完整正文，不再发送一份重复正文。
        chapter.remove("content");
    }
    let has_blueprint = project_context.get("blueprint").is_some();
    if let Some(project) = project_context
        .get_mut("project_detail")
        .and_then(serde_json::Value::as_object_mut)
    {
        project.remove("export_config");
        if has_blueprint {
            project.remove("raw_blueprint");
        }
    }
    let context_hash = source_content_sha256(&serde_json::to_string(&project_context)?);
    ledger.add_pack(a4n_llm_context::LlmContextLedgerPackInput {
        scope_kind: ("chapter").into(),
        pack_kind: ("optimization_patch_project_context").into(),
        source_refs_json: Some(json!({
            "source": "content_optimization_context",
            "content_sha256": context_hash,
        })),
        config_identity_json: json!({
            "ledger_version": "optimization_patch_context_v1",
            "kind": "project_context",
        }),
        required: true,
        stable_prefix: true,
        variants: vec![LlmContextLedgerPackDraft::new(
            COMPRESSION_FULL_RAW,
            json!({"kind": "project_context", "context": project_context}),
        )],
    })?;

    Ok(ledger.into_manifest(
        dynamic_user_content,
        Some(
            "内容优化补丁必须返回严格 JSON 对象；只允许 summary 和 patches，patches 只能包含局部 replace/delete/insert_before/insert_after 补丁。"
                .to_string(),
        ),
    ))
}
