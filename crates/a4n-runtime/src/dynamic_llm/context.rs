use super::*;

pub(super) type ContextContentByRef = HashMap<(String, String, String), String>;

pub(super) async fn build_manifest_requirements(
    orchestrator: &OrchestratorContext,
    provider: &dyn LlmProvider,
    trace_context: &LlmTraceContext,
    manifest: &LlmContextManifest,
) -> Result<(Vec<LlmContextPackRequirement>, ContextContentByRef), AppError> {
    let project_id = trace_context.project_id.ok_or_else(|| {
        AppError::InvalidArgument(format!(
            "LLM trace 缺少 project_id，无法记录上下文 manifest：workflow={}, step={}",
            trace_context.workflow, trace_context.step_key
        ))
    })?;
    let root_project_id = trace_context.root_project_id.unwrap_or(project_id);
    let mut requirements = Vec::new();
    let mut content_by_ref = HashMap::new();

    for pack in &manifest.packs {
        let mut variants = Vec::new();
        for variant in &pack.variants {
            let draft = LlmContextPackDraft::from_json(LlmContextPackDraftInput {
                project_id,
                root_project_id,
                scope_kind: pack.scope_kind.clone(),
                workflow: trace_context.workflow.clone(),
                pack_kind: pack.pack_kind.clone(),
                source_refs_json: pack.source_refs_json.clone(),
                config_hash: pack.config_hash.clone(),
                compression_level: variant.compression_level.clone(),
                canonical_content_json: variant.canonical_content_json.clone(),
            })?;
            let canonical_content_text = draft.canonical_content_text.clone();
            let model = record_or_reuse_context_pack_for_llm(orchestrator, draft).await?;
            let mut pack_ref = context_pack_model_ref(&model);
            pack_ref.token_estimate = provider.count_tokens(&canonical_content_text);
            content_by_ref.insert(selected_pack_key(&pack_ref), canonical_content_text);
            variants.push(pack_ref);
        }
        requirements.push(LlmContextPackRequirement {
            pack_kind: pack.pack_kind.clone(),
            required: pack.required,
            stable_prefix: pack.stable_prefix,
            variants,
        });
    }

    Ok((requirements, content_by_ref))
}

pub(super) fn hydrate_selected_context_pack_content(
    selected_packs: &mut [PlannedContextPack],
    content_by_ref: &ContextContentByRef,
) {
    for selected in selected_packs {
        selected.canonical_content_text = content_by_ref
            .get(&selected_pack_key(&selected.pack_ref))
            .cloned();
    }
}

pub(super) fn selected_pack_key(pack_ref: &LlmContextPackRef) -> (String, String, String) {
    (
        pack_ref.pack_kind.clone(),
        pack_ref.compression_level.clone(),
        pack_ref.content_sha256.clone(),
    )
}

pub(super) fn system_messages(messages: &[ChatMessage]) -> Vec<ChatMessage> {
    messages
        .iter()
        .filter(|message| matches!(&message.role, MessageRole::System))
        .cloned()
        .collect()
}

pub(super) fn non_system_messages(messages: &[ChatMessage]) -> Vec<ChatMessage> {
    messages
        .iter()
        .filter(|message| !matches!(&message.role, MessageRole::System))
        .cloned()
        .collect()
}

pub(super) async fn build_system_prefix_pack_ref(
    orchestrator: &OrchestratorContext,
    trace_context: &LlmTraceContext,
    system_messages: &[ChatMessage],
    token_estimate: usize,
) -> Result<LlmContextPackRef, AppError> {
    let content = json!({
        "schema_version": 1,
        "messages": system_messages,
    });
    let project_id = trace_context.project_id.ok_or_else(|| {
        AppError::InvalidArgument(format!(
            "LLM trace 缺少 project_id，无法记录上下文包：workflow={}, step={}",
            trace_context.workflow, trace_context.step_key
        ))
    })?;
    let root_project_id = trace_context.root_project_id.unwrap_or(project_id);
    let draft = LlmContextPackDraft::from_json(LlmContextPackDraftInput {
        project_id,
        root_project_id,
        scope_kind: "llm_request".to_string(),
        workflow: trace_context.workflow.clone(),
        pack_kind: "system_prompt_prefix".to_string(),
        source_refs_json: Some(json!({
            "source": "chat_request.system_messages",
            "step_key": trace_context.step_key.clone(),
        })),
        config_hash: "system_prompt_prefix:v1".to_string(),
        compression_level: COMPRESSION_FULL_RAW.to_string(),
        canonical_content_json: content,
    })?;
    let model = record_or_reuse_context_pack_for_llm(orchestrator, draft).await?;
    let mut pack_ref = context_pack_model_ref(&model);
    pack_ref.token_estimate = token_estimate;
    Ok(pack_ref)
}

pub(super) fn context_pack_record_input(
    draft: LlmContextPackDraft,
) -> RecordOrReuseContextPackInput {
    RecordOrReuseContextPackInput {
        project_id: draft.project_id,
        root_project_id: draft.root_project_id,
        scope_kind: draft.scope_kind,
        workflow: draft.workflow,
        pack_kind: draft.pack_kind,
        source_refs_json: draft.source_refs_json,
        config_hash: draft.config_hash,
        content_sha256: draft.content_sha256,
        canonical_content_json: draft.canonical_content_json,
        canonical_content_text: draft.canonical_content_text,
        compression_level: draft.compression_level,
        token_estimate: draft.token_estimate,
    }
}

pub(super) async fn record_or_reuse_context_pack_for_llm(
    orchestrator: &OrchestratorContext,
    draft: LlmContextPackDraft,
) -> Result<a4n_core::models::generation::llm_context_pack::Model, AppError> {
    orchestrator
        .record_or_reuse_context_pack(context_pack_record_input(draft))
        .await
}
