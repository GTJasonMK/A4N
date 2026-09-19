use a4n_agent_runtime_contract::AgentRunContext;
use a4n_api_contract::agent::AgentSkillSearchResult;
use a4n_error::AppError;
use a4n_llm_context::{
    COMPRESSION_FOCUSED_RAW, COMPRESSION_FULL_RAW, COMPRESSION_MINIMAL, LlmContextLedger,
    LlmContextLedgerPackDraft, LlmContextLedgerPackInput, LlmContextManifest,
};
use a4n_llm_provider_contract::{ChatMessage, MessageRole};
use serde_json::{Value, json};

const CONFIG_IDENTITY: &str = "agent-runtime-context:v1";

pub(super) fn build_manifest(
    context: &AgentRunContext,
    messages: &[ChatMessage],
    skills: &AgentSkillSearchResult,
) -> Result<Option<LlmContextManifest>, AppError> {
    let mut ledger = LlmContextLedger::new();
    add_pack(
        &mut ledger,
        context,
        "agent_history_units",
        json!({
            "run_id": context.run.id,
            "turn_id": context.turn.id,
            "message_count": messages.len(),
            "roles": messages.iter().map(|message| role_name(&message.role)).collect::<Vec<_>>(),
            "tool_call_ids": messages
                .iter()
                .flat_map(|message| message.tool_calls.as_deref().unwrap_or_default())
                .map(|call| call.id.clone())
                .collect::<Vec<_>>(),
        }),
        Some(json!({"run_id": context.run.id, "turn_id": context.turn.id})),
        true,
        false,
    )?;

    if let Some(summary) = context.thread.context_summary.as_ref() {
        add_pack(
            &mut ledger,
            context,
            "agent_thread_summary",
            summary.clone(),
            Some(json!({"thread_id": context.thread.id, "revision": context.thread.revision})),
            false,
            false,
        )?;
    }

    if !skills.skills.is_empty() {
        add_pack(
            &mut ledger,
            context,
            "agent_skill_directory",
            serde_json::to_value(&skills.skills)?,
            Some(json!({"source": "search_agent_skills", "total": skills.total})),
            false,
            false,
        )?;
    }

    let observation_refs = context
        .executions
        .iter()
        .filter(|execution| execution.execution_kind != "model_call")
        .take(64)
        .map(|execution| {
            json!({
                "execution_id": execution.id.to_string(),
                "tool_name": execution.tool_name,
                "status": execution.status,
                "result_refs": execution.result_refs_json,
                "result_summary": execution.result_summary,
            })
        })
        .collect::<Vec<_>>();
    if !observation_refs.is_empty() {
        add_pack(
            &mut ledger,
            context,
            "agent_observation_refs",
            Value::Array(observation_refs),
            Some(json!({"run_id": context.run.id})),
            false,
            false,
        )?;
    }

    let dynamic_user_content = messages
        .iter()
        .rev()
        .find(|message| matches!(message.role, MessageRole::User))
        .map(|message| message.content.clone())
        .filter(|content| !content.trim().is_empty())
        .or_else(|| context.goal.clone())
        .unwrap_or_else(|| "继续执行当前 Agent 任务".into());

    Ok(Some(ledger.into_manifest(dynamic_user_content, None)))
}

fn add_pack(
    ledger: &mut LlmContextLedger,
    context: &AgentRunContext,
    pack_kind: &str,
    full_content: Value,
    source_refs_json: Option<Value>,
    required: bool,
    stable_prefix: bool,
) -> Result<(), AppError> {
    let minimal_content = json!({"pack_kind": pack_kind, "available": true});
    ledger.add_pack(LlmContextLedgerPackInput {
        scope_kind: "agent_run".into(),
        pack_kind: pack_kind.into(),
        source_refs_json,
        config_identity_json: json!({
            "owner": CONFIG_IDENTITY,
            "run_id": context.run.id,
            "thread_id": context.thread.id,
        }),
        required,
        stable_prefix,
        variants: vec![
            LlmContextLedgerPackDraft::new(COMPRESSION_FULL_RAW, full_content.clone()),
            LlmContextLedgerPackDraft::new(COMPRESSION_FOCUSED_RAW, full_content),
            LlmContextLedgerPackDraft::new(COMPRESSION_MINIMAL, minimal_content),
        ],
    })
}

fn role_name(role: &MessageRole) -> &'static str {
    match role {
        MessageRole::System => "system",
        MessageRole::User => "user",
        MessageRole::Assistant => "assistant",
        MessageRole::Tool => "tool",
    }
}

#[cfg(test)]
mod tests {
    use super::build_manifest;
    use a4n_agent_runtime_contract::{
        AgentDelegationSnapshot, AgentExecutionSnapshot, AgentMessageSnapshot, AgentRunContext,
        AgentRunSnapshot, AgentThreadSnapshot, AgentTurnSnapshot,
    };
    use a4n_api_contract::agent::{AgentScope, AgentSkillSearchResult};
    use a4n_llm_provider_contract::ChatMessage;
    use serde_json::json;

    fn context(project_id: Option<i64>) -> AgentRunContext {
        AgentRunContext {
            run: AgentRunSnapshot {
                id: 10,
                thread_id: 20,
                turn_id: 30,
                parent_run_id: None,
                attempt: 1,
                status: "running".into(),
                budget_json: json!({}),
                usage_json: json!({}),
                worker_id: Some("test".into()),
                lease_token: Some("lease".into()),
                lease_expires_at: None,
                created_at: chrono::Utc::now().naive_utc(),
                started_at: None,
            },
            thread: AgentThreadSnapshot {
                id: 20,
                owner_user_id: 1,
                thread_kind: "main".into(),
                profile_id: "main".into(),
                title: None,
                project_id,
                resource_type: None,
                resource_id: None,
                scope_json: json!({}),
                default_model_config_id: None,
                status: "active".into(),
                revision: 1,
                context_summary: Some(json!({"current_goal": "测试"})),
            },
            turn: AgentTurnSnapshot {
                id: 30,
                thread_id: 20,
                input_message_id: 40,
                execution_mode: "chat".into(),
                goal: Some("测试目标".into()),
                model_override_json: None,
            },
            messages: Vec::<AgentMessageSnapshot>::new(),
            delegations: Vec::<AgentDelegationSnapshot>::new(),
            executions: vec![AgentExecutionSnapshot {
                id: 50,
                run_id: 10,
                execution_key: "tool-call:read".into(),
                execution_kind: "tool_call".into(),
                status: "succeeded".into(),
                tool_name: Some("read_project".into()),
                effect: Some("read".into()),
                replay_policy: Some("safe".into()),
                arguments_json: Some(json!({})),
                resource_preconditions_json: None,
                result_json: Some(json!({"ok": true})),
                result_summary: Some("read".into()),
                result_refs_json: Some(json!([])),
                error_message: None,
            }],
            editor_source: None,
            owner_user_id: 1,
            scope: AgentScope {
                project_id,
                chapter_id: None,
                resource_kind: None,
                resource_id: None,
                allowed_capabilities: Vec::new(),
                allowed_tools: Vec::new(),
            },
            profile_id: "main".into(),
            default_model_config_id: None,
            execution_mode: "chat".into(),
            goal: Some("测试目标".into()),
            budget: json!({}),
            usage: json!({}),
            runtime_snapshot_required: false,
            delegation_depth: 0,
        }
    }

    #[test]
    fn global_agents_build_local_context_packs_without_project_scope() {
        let manifest = build_manifest(
            &context(None),
            &[ChatMessage::user("继续")],
            &AgentSkillSearchResult {
                skills: Vec::new(),
                total: 0,
            },
        )
        .unwrap();
        let manifest = manifest.unwrap();
        assert!(
            manifest
                .packs
                .iter()
                .any(|pack| pack.pack_kind == "agent_history_units")
        );
    }

    #[test]
    fn scoped_manifest_contains_history_summary_and_observation_refs() {
        let manifest = build_manifest(
            &context(Some(7)),
            &[ChatMessage::user("继续")],
            &AgentSkillSearchResult {
                skills: Vec::new(),
                total: 0,
            },
        )
        .unwrap()
        .unwrap();
        assert_eq!(manifest.schema_version, 1);
        assert_eq!(manifest.dynamic_user_content, "继续");
        assert!(
            manifest
                .packs
                .iter()
                .any(|pack| pack.pack_kind == "agent_history_units" && pack.required)
        );
        assert!(
            manifest
                .packs
                .iter()
                .any(|pack| pack.pack_kind == "agent_thread_summary")
        );
        assert!(
            manifest
                .packs
                .iter()
                .any(|pack| pack.pack_kind == "agent_observation_refs")
        );
    }
}
