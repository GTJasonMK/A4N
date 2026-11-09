use crate::host::AgentRuntimeHost;
use crate::state::AgentRuntimeState;
use a4n_agent_runtime_contract::{
    AgentExecutionSnapshot, AgentInboxSnapshot, AgentMessageSnapshot, AgentModelDecision,
    AgentRunContext, AgentWorkerApprovalApplicationPort, AgentWorkerBudgetApplicationPort,
    AgentWorkerExecutionApplicationPort, AgentWorkerFailureApplicationPort,
    AgentWorkerMessageApplicationPort, AgentWorkerRun, WorkerDelegationInput, WorkerExecutionInput,
    WorkerExecutionUpdate, agent_protocol_from_request_snapshot, agent_protocol_prompt,
    agent_skill_directory_prompt, child_budget_for_delegation, configure_agent_request,
    decode_agent_response, enforce_delegation_limits, ensure_agent_scope_allows_tool,
    restore_model_request_wire, validate_agent_budget,
};
use a4n_api_contract::agent::{
    AgentBudget, AgentScope, AgentSkillSearchResult, AgentToolDescriptor, DescribeAgentToolInput,
    LoadAgentSkillInput, SearchAgentSkillsInput, SearchAgentToolsInput,
};
use a4n_error::AppError;
use a4n_llm_provider_contract::{
    ChatMessage, ChatRequest, ChatResponse, ChatStreamAccumulator, LlmTraceContext, MessageRole,
    ToolCall, validated_model_output_tokens,
};
use a4n_worker_contract::{
    AGENT_RUN_OPERATION, AgentRunPayload, AgentWorkerApprovalExpiryRequest,
    AgentWorkerChapterScopeRequest, AgentWorkerClaimRequest, AgentWorkerDelegationScopeRequest,
    AgentWorkerEditorWaitExpiryRequest, AgentWorkerExpiredRunRecoveryRequest,
    AgentWorkerExternalTaskWaitSnapshotWire, AgentWorkerRunWire, AgentWorkerToolDescribeRequest,
    AgentWorkerToolSearchRequest, WORKER_SCHEMA_VERSION, WorkerEnvelope, WorkerLease,
    WorkerPrincipal, WorkerPrincipalKind,
};
use futures::StreamExt;
use serde::Deserialize;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::{Mutex, Notify};
use tokio::time::{Duration, MissedTickBehavior};
use tokio_util::sync::CancellationToken;

const WORKER_POLL_INTERVAL: Duration = Duration::from_secs(2);
const WORKER_RENEW_INTERVAL: Duration = Duration::from_secs(10);
const WORKER_MAX_CONCURRENT_RUNS: usize = 4;
const WORKER_APPROVAL_TTL_SECONDS: i64 = 15 * 60;
const MAX_DISCOVERED_AGENT_TOOLS: usize = 8;
// Discovery remains available even when a domain's full catalog does not fit.
const MAX_AGENT_TOOL_SCHEMAS: usize = 32;
const AGENT_STREAM_PROJECTION_INTERVAL: Duration = Duration::from_millis(100);
const AGENT_STREAM_PROJECTION_MAX_CHARS: usize = 256;
pub(super) const MAX_OBSERVATION_BYTES: usize = 16 * 1024;

#[path = "loop.rs"]
mod agent_loop;
mod binding;
mod context;

mod editor;
mod history;
mod model;
mod observation;
mod prompt;
mod stream;
mod task_wait;
mod tool_calls;
mod tool_window;
mod tools;
mod usage;
mod wire;

pub use agent_loop::{
    claim_agent_worker_request, execute_claimed_worker_run, maintain_agent_worker, run_worker,
};
pub use observation::{bounded_observation, tool_error_code};
pub use tool_calls::{append_tool_observation, is_correctable_tool_error};
pub use tool_window::{enum_string, json_u64};
pub use tools::{
    collect_argument_chapter_targets, collect_scope_ids, editor_request_for_call,
    ensure_argument_scope, ensure_chapter_scope, ensure_project_scope, parse_arguments,
    should_request_approval, tool_descriptor, tool_result_refs,
};
pub use wire::agent_worker_run_from_wire;

pub(super) use agent_loop::RunProgress;
pub(super) use context::{compact_context, execution_input_message_id, recover_model_messages};
pub(super) use model::{
    model_execution_request, model_execution_snapshot, model_response, stream_client_message_id,
};
pub(super) use prompt::{chat_message_from_model, chat_messages, with_agent_system_prompt};
pub(super) use stream::{complete_run, consume_model_stream};
pub(super) use tool_calls::{ToolBatchContext, append_inbox_messages, execute_tool_calls};
pub(super) use tool_window::{
    available_runtime_tools, discovered_tool_names, discovery_key, extend_discovered_tools,
    is_discovery_tool, record_tool_failure, successful_discovery_cache, usage_object,
};
pub(super) use tools::{RuntimeAgentToolExecutor, ensure_agent_package_enabled};
pub(super) use usage::{persist_usage, record_provider_failure};
pub(super) use wire::validate_claimed_worker_request;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn optional_discovered_tools_are_soft_limited_to_the_schema_window() {
        let scope = AgentScope {
            resource_kind: None,
            resource_id: None,
            project_id: None,
            chapter_id: None,
            chapter_number: None,
            allowed_capabilities: Vec::new(),
            allowed_tools: Vec::new(),
        };
        let mut discovered = Vec::new();
        for capability in [
            a4n_api_contract::agent::DIRECTOR_AGENT_CAPABILITY,
            a4n_api_contract::agent::CANVAS_AGENT_CAPABILITY,
            a4n_api_contract::agent::CONTENT_OPTIMIZATION_AGENT_CAPABILITY,
        ] {
            discovered.extend(
                a4n_api_contract::agent::agent_tools_for_capability(capability)
                    .iter()
                    .map(|name| (*name).to_string()),
            );
        }
        let tools = available_runtime_tools(
            a4n_api_contract::agent::MAIN_AGENT_CAPABILITY,
            &scope,
            &discovered,
        )
        .expect("optional discovered tools should be trimmed");
        assert!(tools.schemas.len() <= MAX_AGENT_TOOL_SCHEMAS);
        assert!(tools.dropped_optional.len() <= discovered.len());
    }

    #[test]
    fn director_initial_tools_are_not_evicted_by_discovery() {
        let tools = available_runtime_tools(
            a4n_api_contract::agent::DIRECTOR_AGENT_CAPABILITY,
            &AgentScope::default(),
            &a4n_api_contract::agent::DIRECTOR_AGENT_TOOLS
                .iter()
                .map(|name| (*name).to_string())
                .collect::<Vec<_>>(),
        )
        .expect("director tool window should resolve");
        let names = tools
            .schemas
            .iter()
            .filter_map(|schema| schema["function"]["name"].as_str())
            .collect::<HashSet<_>>();
        for name in a4n_api_contract::agent::initial_agent_tools(
            a4n_api_contract::agent::DIRECTOR_AGENT_CAPABILITY,
        ) {
            assert!(
                names.contains(name),
                "missing Director initial tool: {name}"
            );
        }
    }

    #[test]
    fn stream_projection_starts_on_first_delta_and_throttles_followup_writes() {
        assert!(stream::should_flush_stream_projection(
            true,
            false,
            0,
            0,
            Duration::from_millis(1),
        ));
        assert!(stream::should_flush_stream_projection(
            true,
            true,
            256,
            0,
            Duration::from_millis(1),
        ));
        assert!(stream::should_flush_stream_projection(
            true,
            true,
            1,
            0,
            AGENT_STREAM_PROJECTION_INTERVAL,
        ));
        assert!(!stream::should_flush_stream_projection(
            false,
            false,
            0,
            0,
            Duration::from_secs(1),
        ));
    }

    #[test]
    fn failed_list_results_do_not_get_parsed_as_success_arrays() {
        for tool_name in ["list_optimize_turns", "list_optimize_patch_sets"] {
            let refs = tool_result_refs(tool_name, &json!({"error": "读取失败"}))
                .expect("failed tool results should not require success parsing");
            assert!(refs.is_empty(), "{tool_name}");
        }
    }

    pub(super) fn approval_context(execution_mode: &str) -> AgentRunContext {
        let now = chrono::Utc::now().naive_utc();
        AgentRunContext {
            run: a4n_agent_runtime_contract::AgentRunSnapshot {
                id: 1,
                thread_id: 2,
                turn_id: 3,
                parent_run_id: None,
                attempt: 1,
                status: "running".into(),
                budget_json: json!({}),
                usage_json: json!({}),
                worker_id: Some("test".into()),
                lease_token: Some("lease".into()),
                lease_expires_at: None,
                created_at: now,
                started_at: Some(now),
            },
            thread: a4n_agent_runtime_contract::AgentThreadSnapshot {
                id: 2,
                owner_user_id: 7,
                thread_kind: "subagent".into(),
                profile_id: "content_optimization".into(),
                title: None,
                project_id: Some(11),
                resource_type: Some("chapter".into()),
                resource_id: Some("13".into()),
                scope_json: json!({}),
                default_model_config_id: None,
                status: "active".into(),
                revision: 1,
                context_summary: None,
            },
            turn: a4n_agent_runtime_contract::AgentTurnSnapshot {
                id: 3,
                thread_id: 2,
                input_message_id: 4,
                execution_mode: execution_mode.into(),
                goal: None,
                model_override_json: None,
            },
            messages: Vec::new(),
            delegations: Vec::new(),
            executions: Vec::new(),
            editor_source: None,
            owner_user_id: 7,
            scope: AgentScope {
                resource_kind: Some("chapter".into()),
                resource_id: Some("13".into()),
                project_id: Some(11),
                chapter_id: Some(13),
                chapter_number: None,
                allowed_capabilities: vec!["content_optimization".into()],
                allowed_tools: Vec::new(),
            },
            profile_id: "content_optimization".into(),
            default_model_config_id: None,
            execution_mode: execution_mode.into(),
            goal: None,
            budget: json!({}),
            usage: json!({}),
            runtime_snapshot_required: false,
            delegation_depth: 0,
        }
    }

    #[test]
    fn article_proposals_do_not_block_before_the_apply_boundary() {
        let review = approval_context("review_writes");
        assert!(!should_request_approval(&review, "edit_article", "write").unwrap());
        assert!(should_request_approval(&review, "apply_optimization_patches", "write").unwrap());

        let advice_only = approval_context("advice_only");
        assert!(should_request_approval(&advice_only, "edit_article", "write").is_err());
    }

    #[test]
    fn director_draft_writes_follow_the_selected_execution_mode() {
        for name in [
            "save_chapter_visual_plan_draft",
            "edit_chapter_visual_plan_draft",
            "save_editor_draft",
        ] {
            assert!(
                !should_request_approval(&approval_context("autonomous"), name, "write").unwrap()
            );
            assert!(
                should_request_approval(&approval_context("review_writes"), name, "write").unwrap()
            );
            assert!(
                should_request_approval(&approval_context("advice_only"), name, "write").is_err()
            );
        }
        assert!(
            should_request_approval(
                &approval_context("autonomous"),
                "confirm_chapter_visual_plan",
                "destructive"
            )
            .unwrap()
        );
        assert!(
            should_request_approval(
                &approval_context("review_writes"),
                "confirm_chapter_visual_plan",
                "destructive"
            )
            .unwrap()
        );
        assert!(
            should_request_approval(
                &approval_context("advice_only"),
                "confirm_chapter_visual_plan",
                "destructive"
            )
            .is_err()
        );
    }
}
