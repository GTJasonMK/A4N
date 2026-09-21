use super::auth::authenticated_user;
use super::error::ApiError;
use crate::state::ServerState;
use a4n_agent_runtime_contract::{
    AgentExternalTaskWaitSnapshot, model_request_wire_value, restore_model_request_wire,
};
use a4n_api_contract::agent::{
    AGENT_EVENTS_DEFAULT_LIMIT, AGENT_EVENTS_MAX_LIMIT, AGENT_THREADS_DEFAULT_LIMIT,
    AGENT_THREADS_MAX_LIMIT, AGENT_TOOLS_DEFAULT_LIMIT, AGENT_TOOLS_MAX_LIMIT, AgentDelegationView,
    AgentEvent, AgentEventsInput, AgentMemoryInput, AgentMemoryScopeKind, AgentMemoryView,
    AgentMessageAccepted, AgentMessageView, AgentRunView, AgentSkillContent,
    AgentSkillSearchResult, AgentThreadSnapshot, AgentThreadView, AgentToolDescriptor,
    AgentToolSearchResult, ArchiveAgentThreadInput, ControlAgentRunInput, ControlChildAgentInput,
    CreateAgentThreadInput, DescribeAgentToolInput, EnsureMainAgentThreadInput,
    ForkAgentThreadInput, ListAgentMessagesInput, ListAgentThreadsInput, ResolveAgentApprovalInput,
    RetractAgentMemoryInput, SearchAgentToolsInput, SendAgentMessageInput,
    SendAgentMessageToChildInput, SpawnAgentInput, UpdateAgentThreadInput,
};
use a4n_api_contract::chapter_visual_plan::VisualPlanProjection;
use a4n_error::AppError;
use a4n_llm_provider_contract::ChatChunk;
use a4n_runtime::host_api::agent_runtime as agent_actions;
use a4n_worker_contract::{
    AGENT_WORKER_AUTH_HEADER, AgentHostCommandPayload, AgentRunPayload,
    AgentWorkerApprovalCheckRequest, AgentWorkerApprovalCheckSnapshotWire,
    AgentWorkerApprovalExpiryRequest, AgentWorkerApprovalRequest, AgentWorkerBudgetRequest,
    AgentWorkerBudgetSnapshotWire, AgentWorkerCancelGenerationRequest,
    AgentWorkerCancelWorkflowRequest, AgentWorkerChapterScopeRequest, AgentWorkerChatRequest,
    AgentWorkerClaimRequest, AgentWorkerCompletionRequest, AgentWorkerCompletionSnapshotWire,
    AgentWorkerContextSummaryRequest, AgentWorkerDelegationReviewRequest,
    AgentWorkerDelegationReviewSnapshotWire, AgentWorkerDelegationScopeRequest,
    AgentWorkerDelegationScopeSnapshotWire, AgentWorkerDelegationStartRequest,
    AgentWorkerDelegationStartSnapshotWire, AgentWorkerDelegationThreadRequest,
    AgentWorkerDelegationThreadSnapshotWire, AgentWorkerEditorProjectionRequest,
    AgentWorkerEditorWaitExpiryRequest, AgentWorkerEditorWaitRequest, AgentWorkerEventRequest,
    AgentWorkerExecutionCreateRequest, AgentWorkerExecutionLookupRequest,
    AgentWorkerExecutionResultRequest, AgentWorkerExecutionSnapshotWire,
    AgentWorkerExecutionUpdateRequest, AgentWorkerExpiredRunRecoveryRequest,
    AgentWorkerExternalTaskWaitScanRequest, AgentWorkerExternalTaskWaitSnapshotWire,
    AgentWorkerExternalTaskWaitWakeRequest, AgentWorkerFailureRequest,
    AgentWorkerGenerationCancelScanRequest, AgentWorkerGenerationCancelSnapshotWire,
    AgentWorkerHistoryRequest, AgentWorkerHistorySnapshotWire, AgentWorkerInboxClaimRequest,
    AgentWorkerInboxConsumeRequest, AgentWorkerInboxMessageRequest, AgentWorkerInboxPersistRequest,
    AgentWorkerInboxPersistSnapshotWire, AgentWorkerInboxSnapshotWire,
    AgentWorkerLlmCapabilitiesRequest, AgentWorkerMemoryRequest, AgentWorkerMessageRequest,
    AgentWorkerMessageSnapshotWire, AgentWorkerPackageExecutionResponse,
    AgentWorkerRecoveryRequest, AgentWorkerResultRefsRequest, AgentWorkerResultRefsSnapshotWire,
    AgentWorkerRunContextWire, AgentWorkerRunLeaseRequest, AgentWorkerRunStatusSnapshotWire,
    AgentWorkerRunWaitRequest, AgentWorkerSkillLoadRequest, AgentWorkerSkillSearchRequest,
    AgentWorkerToolDescribeRequest, AgentWorkerToolSearchRequest, AgentWorkerUsageRequest,
    AgentWorkerWaitRequest, AgentWorkerWorkflowCancelScanRequest,
    AgentWorkerWorkflowCancelSnapshotWire, WorkerRequest,
};
use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, HeaderName, HeaderValue, StatusCode, header};
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::{IntoResponse, Response};
use futures::{Stream, StreamExt, stream};
use serde::Deserialize;
use serde_json::Value;
use std::collections::VecDeque;
use std::convert::Infallible;
use std::time::Duration;

const EVENT_POLL_INTERVAL: Duration = Duration::from_millis(100);

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ThreadListQuery {
    capability: Option<String>,
    #[serde(default)]
    include_archived: bool,
    limit: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct AgentEventsQuery {
    after_sequence: Option<String>,
    limit: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct AgentToolSearchQuery {
    query: Option<String>,
    domain: Option<String>,
    effect: Option<String>,
    capability: Option<String>,
    limit: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct AgentMessagesQuery {
    before: Option<String>,
    limit: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct AgentMemoriesQuery {
    scope_kind: Option<AgentMemoryScopeKind>,
    scope_id: Option<String>,
}

include!("worker_routes.rs");
include!("user_routes.rs");
include!("stream.rs");
include!("helpers.rs");

#[cfg(test)]
mod tests {
    use super::*;
    use a4n_api_contract::agent::AGENT_API_PROTOCOL_VERSION;
    use a4n_llm_provider_contract::{ChatRequest, LlmTraceContext};
    use serde_json::json;

    fn event(thread_id: &str, sequence: u64) -> AgentEvent {
        AgentEvent {
            protocol_version: AGENT_API_PROTOCOL_VERSION,
            thread_id: thread_id.to_string(),
            sequence,
            kind: "message.accepted".to_string(),
            actor_kind: "user".to_string(),
            actor_id: Some("7".to_string()),
            turn_id: Some("11".to_string()),
            run_id: Some("12".to_string()),
            execution_id: None,
            delegation_id: None,
            message_id: Some("13".to_string()),
            payload: Some(json!({"public": true})),
            editor_request: None,
            created_at: "2026-08-27T12:00:00Z".to_string(),
        }
    }

    #[test]
    fn event_cursor_uses_query_or_last_event_id_and_rejects_conflicts() {
        let mut headers = HeaderMap::new();
        assert_eq!(resolve_event_cursor(&headers, Some(4)).unwrap(), Some(4));

        headers.insert("last-event-id", HeaderValue::from_static("7"));
        assert_eq!(resolve_event_cursor(&headers, None).unwrap(), Some(7));
        assert_eq!(resolve_event_cursor(&headers, Some(7)).unwrap(), Some(7));
        assert!(resolve_event_cursor(&headers, Some(6)).is_err());

        headers.insert("last-event-id", HeaderValue::from_static("7:2"));
        assert!(resolve_event_cursor(&headers, None).is_err());
    }

    #[test]
    fn accepts_event_stream_in_a_media_range_list() {
        let mut headers = HeaderMap::new();
        assert!(!accepts_event_stream(&headers).unwrap());
        headers.insert(
            header::ACCEPT,
            HeaderValue::from_static("application/json, text/event-stream; charset=utf-8"),
        );
        assert!(accepts_event_stream(&headers).unwrap());
    }

    #[test]
    fn limits_and_path_body_ids_are_strict() {
        assert_eq!(parse_limit(None, 50, 500).unwrap(), 50);
        assert_eq!(parse_limit(Some("500"), 50, 500).unwrap(), 500);
        assert!(parse_limit(Some("0"), 50, 500).is_err());
        assert!(parse_limit(Some("501"), 50, 500).is_err());
        assert!(parse_limit(Some(" 2"), 50, 500).is_err());

        require_matching_id("42", "42", "thread_id").unwrap();
        assert!(require_matching_id("42", "43", "thread_id").is_err());
        assert!(require_matching_id(" 42", " 42", "thread_id").is_err());
    }

    #[test]
    fn durable_event_batches_must_match_thread_and_increase_sequence() {
        validate_event_batch("9", 2, &[event("9", 3), event("9", 4)]).unwrap();
        assert!(validate_event_batch("9", 3, &[event("9", 3)]).is_err());
        assert!(validate_event_batch("9", 2, &[event("8", 3)]).is_err());
        assert!(validate_event_batch("9", 2, &[event("9", 4), event("9", 3)]).is_err());
    }

    #[test]
    fn sse_payload_uses_sequence_as_id_and_serializes_top_level_event() {
        let event = event("9", 3);
        let (id, data) = agent_event_payload(&event);
        assert_eq!(id, "3");
        let data: serde_json::Value = serde_json::from_str(&data).unwrap();
        assert_eq!(data["sequence"], 3);
        assert_eq!(data["thread_id"], "9");
        assert_eq!(data["kind"], "message.accepted");
    }

    #[test]
    fn agent_chat_payload_accepts_worker_request() {
        let request = ChatRequest::system_user("system", "user", Some(0.2), Some(128))
            .with_json_response_format()
            .with_stream_usage()
            .with_trace_context(LlmTraceContext {
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
        let value = a4n_agent_runtime_contract::model_request_wire_value(&request).unwrap();
        let parsed = restore_model_request_wire(value).unwrap();

        assert_eq!(parsed.messages.len(), 2);
        assert_eq!(parsed.temperature, Some(0.2));
        assert_eq!(parsed.max_tokens, Some(128));
        assert!(parsed.stream_include_usage);
        assert!(parsed.response_format.is_some());
        assert_eq!(
            parsed.trace_context.as_ref().map(|trace| trace.project_id),
            Some(Some(7))
        );
    }
}
