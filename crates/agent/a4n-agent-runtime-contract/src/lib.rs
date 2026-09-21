//! Database-free contracts shared by the Agent control plane and worker.
//!
//! SeaORM rows stay in `a4n-agent-application`; this crate contains only the
//! stable snapshots, worker inputs and ports needed across the process
//! boundary.

use a4n_api_contract::agent::{
    AgentBudget, AgentDelegationView, AgentEditorRequest, AgentExecutionResultPage,
    AgentMemorySearchResult, AgentResourceRef, AgentScope, AgentSkillContent,
    AgentSkillSearchResult, AgentToolDescriptor, AgentToolSearchResult, DescribeAgentToolInput,
    ReadAgentExecutionResultInput, ReadAgentMemoryInput, RespondAgentEditorInput,
    ReviewAgentDelegationInput, SearchAgentToolsInput,
};
use a4n_api_contract::runtime_config::RuntimeConfigSnapshot;
use a4n_domain::agent::AgentWorkflowOrigin;
use a4n_error::AppError;
use a4n_worker_contract::{
    AgentHostCommandPayload, AgentWorkerApprovalCheckRequest, AgentWorkerApprovalCheckSnapshotWire,
    AgentWorkerApprovalRequest, AgentWorkerBudgetRequest, AgentWorkerBudgetSnapshotWire,
    AgentWorkerCompletionRequest, AgentWorkerCompletionSnapshotWire,
    AgentWorkerContextSummaryRequest, AgentWorkerDelegationReviewRequest,
    AgentWorkerDelegationReviewSnapshotWire, AgentWorkerDelegationSnapshotWire,
    AgentWorkerDelegationStartRequest, AgentWorkerDelegationStartSnapshotWire,
    AgentWorkerDelegationThreadRequest, AgentWorkerDelegationThreadSnapshotWire,
    AgentWorkerEditorWaitRequest, AgentWorkerEventRequest, AgentWorkerExecutionCreateRequest,
    AgentWorkerExecutionLookupRequest, AgentWorkerExecutionResultRequest,
    AgentWorkerExecutionSnapshotWire, AgentWorkerExecutionUpdateRequest, AgentWorkerFailureRequest,
    AgentWorkerHistoryRequest, AgentWorkerHistorySnapshotWire, AgentWorkerInboxClaimRequest,
    AgentWorkerInboxConsumeRequest, AgentWorkerInboxMessageRequest, AgentWorkerInboxPersistRequest,
    AgentWorkerInboxPersistSnapshotWire, AgentWorkerInboxSnapshotWire, AgentWorkerMemoryRequest,
    AgentWorkerMessageRequest, AgentWorkerMessageSnapshotWire, AgentWorkerRecoveryRequest,
    AgentWorkerResultRefsRequest, AgentWorkerResultRefsSnapshotWire, AgentWorkerRunContextWire,
    AgentWorkerRunLeaseRequest, AgentWorkerRunSnapshotWire, AgentWorkerRunStatusSnapshotWire,
    AgentWorkerRunWaitRequest, AgentWorkerRunWire, AgentWorkerSkillLoadRequest,
    AgentWorkerSkillSearchRequest, AgentWorkerThreadSnapshotWire, AgentWorkerTurnSnapshotWire,
    AgentWorkerUsageRequest, WorkerEnvelope,
};
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use serde_json::Value;

mod budget;
mod ports;
mod protocol;
mod snapshots;

pub use a4n_api_contract::agent::AgentEditorSource;
pub use budget::*;
pub use ports::*;
pub use protocol::*;
pub use snapshots::*;

#[cfg(test)]
mod tests {
    use super::*;
    use a4n_llm_provider_contract::provider::{ChatRequest, LlmTraceContext};
    use serde_json::json;

    #[test]
    fn child_budget_is_capped_by_parent_remaining_usage() {
        let parent = AgentBudget {
            max_steps: Some(10),
            max_model_calls: Some(8),
            max_tokens: Some(1_000),
            max_delegation_depth: Some(2),
            ..AgentBudget::default()
        };
        let child = AgentBudget {
            max_steps: Some(99),
            max_model_calls: Some(99),
            max_tokens: Some(9_999),
            max_delegation_depth: Some(9),
            ..AgentBudget::default()
        };

        let capped = child_budget_within_parent(
            child,
            &parent,
            &json!({
                "steps_used": 3,
                "model_calls_used": 2,
                "tokens_used": 250
            }),
        )
        .expect("child budget must be capped");
        assert_eq!(capped.max_steps, Some(7));
        assert_eq!(capped.max_model_calls, Some(6));
        assert_eq!(capped.max_tokens, Some(750));
        assert_eq!(capped.max_delegation_depth, Some(1));

        let exhausted = child_budget_within_parent(
            AgentBudget::default(),
            &AgentBudget {
                max_steps: Some(3),
                ..AgentBudget::default()
            },
            &json!({"steps_used": 3}),
        )
        .expect_err("an exhausted parent budget must reject another child");
        assert!(exhausted.to_string().contains("max_steps"));
    }

    #[test]
    fn context_summary_rejects_invalid_input() {
        let invalid = json!({
            "current_goal": "goal",
            "constraints": [],
            "decisions": [],
            "facts_resources": [],
            "completed": [],
            "pending": [],
            "tool_state": [],
            "covered_through_sequence": -1
        });
        assert!(parse_agent_context_summary(Some(&invalid.to_string())).is_err());

        let valid = AgentContextSummary {
            current_goal: "goal".into(),
            constraints: vec!["keep the contract stable".into()],
            decisions: Vec::new(),
            facts_resources: Vec::new(),
            completed: Vec::new(),
            pending: vec!["run the worker check".into()],
            tool_state: Vec::new(),
            covered_through_sequence: 4,
        };
        assert!(valid.validate().is_ok());
    }

    #[test]
    fn deterministic_context_summary_preserves_shape_without_model_output() {
        let summary = deterministic_context_summary(
            Some(&json!({
                "current_goal": "继续阅读正文",
                "constraints": ["保持人物声音"],
                "pending": ["读取下一段"],
            })),
            Some("fallback goal"),
            "{\"tool\":\"read_project_context\",\"result\":true}",
            12,
        )
        .expect("deterministic summary must validate");

        assert_eq!(summary.current_goal, "继续阅读正文");
        assert_eq!(summary.constraints, ["保持人物声音"]);
        assert_eq!(summary.pending, ["读取下一段"]);
        assert_eq!(summary.covered_through_sequence, 12);
        assert!(
            summary
                .tool_state
                .iter()
                .any(|value| value.contains("确定性压缩"))
        );
    }

    #[test]
    fn skill_runtime_helpers_keep_catalog_behavior_in_one_place() {
        assert!(a4n_api_contract::agent::is_agent_skill_tool(
            "search_agent_skills"
        ));
        assert!(!a4n_api_contract::agent::is_agent_skill_tool(
            "read_agent_memory"
        ));
        assert_eq!(
            agent_skill_directory_prompt(&[]),
            "当前 capability 没有启用 Skill。"
        );
        let restricted = AgentScope {
            allowed_tools: vec!["read_agent_memory".into()],
            ..AgentScope::default()
        };
        assert!(ensure_agent_scope_allows_tool(&restricted, "load_agent_skill").is_ok());
        assert!(ensure_agent_scope_allows_tool(&restricted, "list_projects").is_err());
    }

    #[test]
    fn model_request_wire_round_trips_trace_and_context_manifest() {
        let mut request = ChatRequest::system_user("system", "user", Some(0.2), Some(128));
        request.trace_context = Some(LlmTraceContext {
            project_id: Some(7),
            root_project_id: Some(3),
            workflow: "agent".into(),
            step_key: "model".into(),
            generation_run_id: None,
            workflow_run_id: None,
            call_key: Some("call-1".into()),
            input_fingerprint_sha256: None,
            context_plan_json: None,
            context_pack_refs_json: None,
        });
        request.context_manifest_json = Some(json!({"packs": ["durable-only"]}));

        let wire = model_request_wire_value(&request).expect("request must serialize");
        assert_eq!(wire["trace_context"]["project_id"], 7);
        assert_eq!(
            wire["context_manifest_json"],
            json!({"packs": ["durable-only"]})
        );
        let snapshot = restore_model_request_wire(request.snapshot_value().unwrap())
            .expect("durable snapshot must restore");
        assert_eq!(
            snapshot.context_manifest_json,
            Some(json!({"packs": ["durable-only"]}))
        );

        let restored = restore_model_request_wire(wire).expect("wire must restore");
        assert_eq!(
            restored
                .trace_context
                .as_ref()
                .map(|trace| trace.project_id),
            Some(Some(7))
        );
        assert_eq!(
            restored.context_manifest_json,
            Some(json!({"packs": ["durable-only"]}))
        );
    }

    #[test]
    fn provider_503_is_classified_without_hiding_other_failures() {
        assert_eq!(
            provider_failure_class(&AppError::LlmError(
                "LLM API 返回错误 503：no healthy upstream".into()
            )),
            Some(AGENT_FAILURE_PROVIDER_UNAVAILABLE)
        );
        assert_eq!(
            provider_failure_class(&AppError::LlmError("模型输出被 max_tokens 截断".into())),
            None
        );
    }
}
