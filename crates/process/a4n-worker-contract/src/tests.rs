use a4n_api_contract::agent::{
    AgentEditorRequest, AgentScope, DescribeAgentToolInput, LoadAgentSkillInput,
    ReviewAgentDelegationInput, SearchAgentSkillsInput, SearchAgentToolsInput,
};

use super::*;

fn valid_request() -> WorkerEnvelope<serde_json::Value> {
    WorkerEnvelope {
        schema_version: WORKER_SCHEMA_VERSION,
        request_id: "request-1".into(),
        trace_id: "trace-1".into(),
        principal: WorkerPrincipal {
            kind: WorkerPrincipalKind::Service,
            subject: "control".into(),
        },
        operation_id: "project.generate".into(),
        idempotency_key: "idem-1".into(),
        project_id: Some(7),
        root_project_id: Some(7),
        target_revision: Some(3),
        runtime_config_snapshot_ref: Some("snapshot-1".into()),
        lease: WorkerLease {
            worker_id: "worker-1".into(),
            epoch: 2,
            fence_token: "fence-2".into(),
            expires_at: "2030-01-01T00:00:00Z".into(),
        },
        payload: serde_json::json!({"command": "run"}),
    }
}

#[test]
fn valid_request_round_trips_and_validates() {
    let request = valid_request();
    request.validate().expect("fixture must be valid");
    let encoded = serde_json::to_string(&request).expect("request should serialize");
    let decoded: WorkerEnvelope<serde_json::Value> =
        serde_json::from_str(&encoded).expect("request should deserialize");
    assert_eq!(decoded, request);
}

#[test]
fn stale_schema_is_rejected_without_echoing_payload() {
    let mut request = valid_request();
    request.schema_version += 1;
    let error = request.validate().expect_err("schema drift must fail");
    assert_eq!(
        error,
        WorkerContractError::SchemaVersion {
            expected: WORKER_SCHEMA_VERSION,
            actual: WORKER_SCHEMA_VERSION + 1,
        }
    );
    assert!(!error.to_string().contains("command"));
}

#[test]
fn lease_and_project_relations_are_checked_at_the_boundary() {
    let mut request = valid_request();
    request.lease.epoch = 0;
    assert!(matches!(
        request.validate(),
        Err(WorkerContractError::InvalidValue {
            field: "lease.epoch",
            ..
        })
    ));

    let mut request = valid_request();
    request.project_id = None;
    assert_eq!(
        request.validate().expect_err("root project needs project"),
        WorkerContractError::InvalidRelation("root_project_id requires project_id")
    );
}

#[test]
fn unknown_fields_are_rejected() {
    let mut value = serde_json::to_value(valid_request()).expect("request should serialize");
    value["unexpected"] = true.into();
    let error = serde_json::from_value::<WorkerEnvelope<serde_json::Value>>(value)
        .expect_err("unknown fields must fail");
    assert!(error.to_string().contains("unknown field"));
}

#[test]
fn host_command_arguments_follow_runtime_snapshot_policy() {
    let payload = AgentHostCommandPayload {
        run_id: 7,
        turn_id: 8,
        scope: AgentScope::default(),
        execution_id: None,
        command: "read_chapter_visual_plan".into(),
        arguments: serde_json::json!([]),
        runtime_config_snapshot: None,
    };
    assert!(matches!(
        payload.validate(),
        Err(WorkerContractError::InvalidValue {
            field: "arguments",
            ..
        })
    ));

    let payload = AgentHostCommandPayload {
        command: "start_image_generation".into(),
        arguments: serde_json::json!({"projectId": 7}),
        runtime_config_snapshot: Some(serde_json::json!({"llm": {}})),
        ..payload
    };
    assert_eq!(
        payload
            .arguments_with_runtime_snapshot()
            .expect("full runtime snapshot should be forwarded")["runtime_config_snapshot"],
        serde_json::json!({"llm": {}})
    );

    let web_search = AgentHostCommandPayload {
        command: "web_search".into(),
        arguments: serde_json::json!({"query": "A4N"}),
        runtime_config_snapshot: Some(serde_json::json!({
            "schema_version": 2,
            "web_search": {"api_key_encrypted": "personal-key"}
        })),
        ..payload.clone()
    };
    assert_eq!(
        web_search
            .arguments_with_runtime_snapshot()
            .expect("personal web search snapshot should be forwarded")["runtime_config_snapshot"]
            ["web_search"]["api_key_encrypted"],
        serde_json::json!("personal-key")
    );

    let mut without_secrets = payload;
    without_secrets.command = "preview_chapter_prompt".into();
    without_secrets.runtime_config_snapshot = Some(serde_json::json!({
        "schema_version": 2,
        "llm": {
            "id": 1,
            "name": "test",
            "provider": "openai",
            "base_url": "https://example.test/v1",
            "api_key_encrypted": "secret",
            "model": "test-model",
            "max_concurrent": 1,
            "timeout_secs": 30,
            "max_context_tokens": 4096,
            "max_output_tokens": 1024
        }
    }));
    assert_eq!(
            without_secrets
                .arguments_with_runtime_snapshot()
                .expect("redacted runtime snapshot should be forwarded")["runtime_config_snapshot"]
                ["llm"]
                .get("api_key_encrypted"),
            None
        );

    let none = without_secrets;
    assert!(
        AgentHostCommandPayload {
            command: "read_chapter_visual_plan".into(),
            ..none.clone()
        }
        .arguments_with_runtime_snapshot()
        .expect("runtime snapshot should be ignored for None commands")
        .get("runtime_config_snapshot")
        .is_none()
    );

    let stale = AgentHostCommandPayload {
        command: "read_chapter_visual_plan".into(),
        arguments: serde_json::json!({
            "chapterId": 11,
            "runtime_config_snapshot": {"llm": {"api_key_encrypted": "stale"}}
        }),
        runtime_config_snapshot: None,
        ..none
    };
    assert_eq!(
        stale
            .arguments_with_runtime_snapshot()
            .expect("stale runtime snapshot should be removed")
            .get("runtime_config_snapshot"),
        None
    );
}

#[test]
fn direct_llm_requests_require_the_current_worker_schema() {
    let capabilities = AgentWorkerLlmCapabilitiesRequest {
        schema_version: WORKER_SCHEMA_VERSION,
        user_id: 7,
        snapshot: None,
        config_id: None,
    };
    capabilities
        .validate()
        .expect("capabilities request should validate");

    let chat = AgentWorkerChatRequest {
        schema_version: WORKER_SCHEMA_VERSION,
        user_id: 7,
        snapshot: None,
        config_id: None,
        request: serde_json::json!({"messages": []}),
    };
    chat.validate().expect("chat request should validate");

    let mut stale = serde_json::to_value(chat).expect("chat request should serialize");
    stale["schema_version"] = serde_json::json!(WORKER_SCHEMA_VERSION - 1);
    let stale: AgentWorkerChatRequest =
        serde_json::from_value(stale).expect("stale request remains well-formed JSON");
    assert!(matches!(
        stale.validate(),
        Err(WorkerContractError::SchemaVersion { .. })
    ));

    let mut missing = serde_json::to_value(capabilities).expect("request should serialize");
    missing
        .as_object_mut()
        .expect("request should be an object")
        .remove("schema_version");
    assert!(serde_json::from_value::<AgentWorkerLlmCapabilitiesRequest>(missing).is_err());
}

#[test]
fn shared_agent_host_dto_round_trips_and_rejects_unknown_fields() {
    let request = AgentWorkerCancelGenerationRequest {
        user_id: 7,
        task_id: "task-1".into(),
        attempt: 2,
    };
    let encoded = serde_json::to_string(&request).expect("DTO should serialize");
    let decoded: AgentWorkerCancelGenerationRequest =
        serde_json::from_str(&encoded).expect("DTO should deserialize");
    assert_eq!(decoded.user_id, request.user_id);
    assert_eq!(decoded.task_id, request.task_id);
    assert_eq!(decoded.attempt, request.attempt);

    let mut value = serde_json::to_value(request).expect("DTO should serialize");
    value["unexpected"] = true.into();
    assert!(serde_json::from_value::<AgentWorkerCancelGenerationRequest>(value).is_err());
}

#[test]
fn generation_cancel_scan_allows_initial_cursor_and_rejects_unknown_fields() {
    let request = AgentWorkerGenerationCancelScanRequest {
        after_task_id: String::new(),
    };
    request.validate().expect("initial cursor should be valid");
    let encoded = serde_json::to_string(&request).expect("scan request should serialize");
    let decoded: AgentWorkerGenerationCancelScanRequest =
        serde_json::from_str(&encoded).expect("scan request should deserialize");
    assert_eq!(decoded.after_task_id, request.after_task_id);

    let mut value = serde_json::to_value(request).expect("scan request should serialize");
    value["unexpected"] = true.into();
    assert!(serde_json::from_value::<AgentWorkerGenerationCancelScanRequest>(value).is_err());
}

#[test]
fn workflow_cancel_scan_rejects_negative_cursor_and_unknown_fields() {
    let request = AgentWorkerWorkflowCancelScanRequest {
        after_workflow_run_id: 0,
    };
    request.validate().expect("initial cursor should be valid");
    assert!(matches!(
        AgentWorkerWorkflowCancelScanRequest {
            after_workflow_run_id: -1
        }
        .validate(),
        Err(WorkerContractError::InvalidValue {
            field: "after_workflow_run_id",
            ..
        })
    ));

    let mut value = serde_json::to_value(request).expect("scan request should serialize");
    value["unexpected"] = true.into();
    assert!(serde_json::from_value::<AgentWorkerWorkflowCancelScanRequest>(value).is_err());
}

#[test]
fn usage_request_requires_an_object_and_round_trips() {
    let request = AgentWorkerUsageRequest {
        run_id: 7,
        lease: valid_request().lease,
        usage_json: serde_json::json!({"tokens_used": 3}),
    };
    request.validate().expect("usage request should be valid");
    let encoded = serde_json::to_string(&request).expect("usage request should serialize");
    let decoded: AgentWorkerUsageRequest =
        serde_json::from_str(&encoded).expect("usage request should deserialize");
    assert_eq!(decoded.run_id, request.run_id);

    let invalid = AgentWorkerUsageRequest {
        usage_json: serde_json::json!([]),
        ..request
    };
    assert!(matches!(
        invalid.validate(),
        Err(WorkerContractError::InvalidValue {
            field: "usage_json",
            ..
        })
    ));
}

#[test]
fn run_wait_request_requires_a_reason() {
    let request = AgentWorkerRunWaitRequest {
        run_id: 7,
        lease: valid_request().lease,
        reason: "external".into(),
    };
    request.validate().expect("wait request should be valid");

    let invalid = AgentWorkerRunWaitRequest {
        reason: "  ".into(),
        ..request
    };
    assert!(matches!(
        invalid.validate(),
        Err(WorkerContractError::BlankField { field: "reason" })
    ));
}

#[test]
fn context_summary_request_requires_an_object() {
    let request = AgentWorkerContextSummaryRequest {
        run_id: 7,
        lease: valid_request().lease,
        context_summary: serde_json::json!({"goal": "draft"}),
    };
    request
        .validate()
        .expect("context summary request should be valid");

    let invalid = AgentWorkerContextSummaryRequest {
        context_summary: serde_json::json!("not-an-object"),
        ..request
    };
    assert!(matches!(
        invalid.validate(),
        Err(WorkerContractError::InvalidValue {
            field: "context_summary",
            ..
        })
    ));
}

#[test]
fn failure_request_requires_an_error_message_and_round_trips() {
    let request = AgentWorkerFailureRequest {
        run_id: 7,
        lease: valid_request().lease,
        error_message: "provider failed".into(),
    };
    request.validate().expect("failure request should be valid");
    let encoded = serde_json::to_string(&request).expect("failure request should serialize");
    let decoded: AgentWorkerFailureRequest =
        serde_json::from_str(&encoded).expect("failure request should deserialize");
    assert_eq!(decoded.run_id, request.run_id);
    assert_eq!(decoded.error_message, request.error_message);

    let invalid = AgentWorkerFailureRequest {
        error_message: "  ".into(),
        ..request
    };
    assert!(matches!(
        invalid.validate(),
        Err(WorkerContractError::BlankField {
            field: "error_message"
        })
    ));
}

#[test]
fn result_refs_request_validates_its_lease_and_snapshot_round_trips() {
    let request = AgentWorkerResultRefsRequest {
        run_id: 7,
        lease: valid_request().lease,
    };
    request
        .validate()
        .expect("result refs request should be valid");
    let snapshot = AgentWorkerResultRefsSnapshotWire {
        result_refs_json: serde_json::json!([{"kind": "project", "id": "7"}]),
    };
    let encoded = serde_json::to_string(&snapshot).expect("snapshot should serialize");
    let decoded: AgentWorkerResultRefsSnapshotWire =
        serde_json::from_str(&encoded).expect("snapshot should deserialize");
    assert_eq!(decoded.result_refs_json, snapshot.result_refs_json);
}

#[test]
fn completion_request_requires_content_and_observed_message_boundary() {
    let request = AgentWorkerCompletionRequest {
        run_id: 7,
        lease: valid_request().lease,
        content: "done".into(),
        result_summary: "done".into(),
        result_refs_json: Some(serde_json::json!([])),
        observed_input_message_id: 9,
    };
    request
        .validate()
        .expect("completion request should be valid");
    let invalid = AgentWorkerCompletionRequest {
        observed_input_message_id: 0,
        ..request
    };
    assert!(matches!(
        invalid.validate(),
        Err(WorkerContractError::InvalidValue {
            field: "observed_input_message_id",
            ..
        })
    ));
}

#[test]
fn approval_requests_validate_execution_and_operation_boundaries() {
    let check = AgentWorkerApprovalCheckRequest {
        run_id: 7,
        lease: valid_request().lease.clone(),
        execution_id: 9,
    };
    check.validate().expect("approval check should be valid");

    let request = AgentWorkerApprovalRequest {
        run_id: check.run_id,
        lease: check.lease,
        execution_id: check.execution_id,
        operation_json: serde_json::json!({"command": "write"}),
        resource_preconditions_json: None,
        expires_at: None,
    };
    request
        .validate()
        .expect("approval request should be valid");
    let invalid = AgentWorkerApprovalRequest {
        operation_json: serde_json::json!("write"),
        ..request
    };
    assert!(matches!(
        invalid.validate(),
        Err(WorkerContractError::InvalidValue {
            field: "operation_json",
            ..
        })
    ));
}

#[test]
fn delegation_review_request_requires_a_positive_user_and_parent_run() {
    let request = AgentWorkerDelegationReviewRequest {
        run_id: 7,
        lease: valid_request().lease,
        user_id: 11,
        parent_run_id: Some(7),
        input: ReviewAgentDelegationInput {
            delegation_id: "3".into(),
            expected_child_run_id: "4".into(),
            decision: a4n_api_contract::agent::AgentDelegationReviewDecision::Accept,
            reason: "verified".into(),
            reviewed_refs: Vec::new(),
        },
    };
    request
        .validate()
        .expect("delegation review request should be valid");
    let invalid = AgentWorkerDelegationReviewRequest {
        user_id: 0,
        ..request
    };
    assert!(matches!(
        invalid.validate(),
        Err(WorkerContractError::InvalidValue {
            field: "user_id",
            ..
        })
    ));
}

#[test]
fn run_status_snapshot_round_trips() {
    let snapshot = AgentWorkerRunStatusSnapshotWire { terminal: true };
    let encoded = serde_json::to_string(&snapshot).expect("run status should serialize");
    let decoded: AgentWorkerRunStatusSnapshotWire =
        serde_json::from_str(&encoded).expect("run status should deserialize");
    assert!(decoded.terminal);
}

#[test]
fn delegation_thread_request_validates_identity_fields() {
    let request = AgentWorkerDelegationThreadRequest {
        run_id: 7,
        lease: valid_request().lease,
        user_id: 11,
        canonical_binding_key: "subagent:director:chapter:7".into(),
        capability: "director".into(),
        title: Some("Director".into()),
        scope: AgentScope {
            resource_kind: Some("chapter".into()),
            resource_id: Some("7".into()),
            project_id: Some(3),
            chapter_id: Some(7),
            allowed_capabilities: vec!["director".into()],
            allowed_tools: Vec::new(),
        },
        bound_resource: None,
        default_model_config_id: Some(4),
    };
    request
        .validate()
        .expect("delegation thread request should be valid");
    let invalid = AgentWorkerDelegationThreadRequest {
        capability: "  ".into(),
        ..request
    };
    assert!(matches!(
        invalid.validate(),
        Err(WorkerContractError::BlankField {
            field: "capability"
        })
    ));
}

#[test]
fn event_request_requires_publishable_fields() {
    let request = AgentWorkerEventRequest {
        run_id: 7,
        lease: valid_request().lease,
        kind: "delegation_started".into(),
        status: "published".into(),
        public_text: "started".into(),
        payload_json: Some(serde_json::json!({"capability": "director"})),
    };
    request.validate().expect("event request should be valid");
    let invalid = AgentWorkerEventRequest {
        public_text: " ".into(),
        ..request
    };
    assert!(matches!(
        invalid.validate(),
        Err(WorkerContractError::BlankField {
            field: "public_text"
        })
    ));
}

#[test]
fn delegation_start_request_validates_transaction_input() {
    let request = AgentWorkerDelegationStartRequest {
        run_id: 7,
        lease: valid_request().lease,
        capability: "director".into(),
        objective: "整理镜头计划".into(),
        delegation_key: "tool-call-1".into(),
        child_thread_id: 9,
        scope: AgentScope {
            resource_kind: Some("chapter".into()),
            resource_id: Some("7".into()),
            project_id: Some(3),
            chapter_id: Some(7),
            allowed_capabilities: vec!["director".into()],
            allowed_tools: Vec::new(),
        },
        success_criteria: vec!["返回可执行方案".into()],
        budget_json: serde_json::json!({"max_steps": 4}),
        parent_execution_id: Some(11),
        runtime_config_snapshot: None,
    };
    request
        .validate()
        .expect("delegation start request should be valid");
    let invalid = AgentWorkerDelegationStartRequest {
        child_thread_id: 0,
        ..request
    };
    assert!(matches!(
        invalid.validate(),
        Err(WorkerContractError::InvalidValue {
            field: "child_thread_id",
            ..
        })
    ));
}

#[test]
fn execution_requests_validate_their_lease_and_identifiers() {
    let lease = valid_request().lease;
    let lookup = AgentWorkerExecutionLookupRequest {
        run_id: 7,
        lease: lease.clone(),
        execution_key: "tool-call:1".into(),
    };
    lookup.validate().expect("lookup should be valid");

    let create = AgentWorkerExecutionCreateRequest {
        run_id: 7,
        lease: lease.clone(),
        execution_key: "tool-call:1".into(),
        execution_kind: "tool_call".into(),
        tool_name: Some("read".into()),
        effect: Some("read".into()),
        replay_policy: Some("safe".into()),
        arguments_json: Some(serde_json::json!({})),
        resource_preconditions_json: None,
    };
    create.validate().expect("create should be valid");

    let update = AgentWorkerExecutionUpdateRequest {
        run_id: 7,
        lease,
        execution_id: 2,
        status: "succeeded".into(),
        result_json: None,
        result_summary: None,
        result_refs_json: None,
        error_message: None,
    };
    update.validate().expect("update should be valid");
    assert!(matches!(
        AgentWorkerExecutionUpdateRequest {
            execution_id: 0,
            ..update
        }
        .validate(),
        Err(WorkerContractError::InvalidValue {
            field: "execution_id",
            ..
        })
    ));

    let result = AgentWorkerExecutionResultRequest {
        run_id: 7,
        lease: valid_request().lease,
        execution_id: "11".into(),
        pointer: "/text".into(),
        offset: 0,
        limit: Some(20),
    };
    result
        .validate()
        .expect("execution result request should be valid");
    assert!(matches!(
        AgentWorkerExecutionResultRequest {
            limit: Some(0),
            ..result
        }
        .validate(),
        Err(WorkerContractError::InvalidValue { field: "limit", .. })
    ));
}

#[test]
fn memory_request_rejects_zero_limit() {
    let request = AgentWorkerMemoryRequest {
        run_id: 7,
        lease: valid_request().lease,
        query: Some("scope".into()),
        limit: Some(10),
    };
    request.validate().expect("memory request should be valid");
    assert!(matches!(
        AgentWorkerMemoryRequest {
            limit: Some(0),
            ..request
        }
        .validate(),
        Err(WorkerContractError::InvalidValue { field: "limit", .. })
    ));
}

#[test]
fn worker_read_requests_validate_boundary_values() {
    let search = AgentWorkerToolSearchRequest {
        input: SearchAgentToolsInput {
            query: None,
            domain: None,
            effect: None,
            capability: None,
            limit: Some(1),
        },
    };
    search.validate().expect("tool search should be valid");
    assert!(matches!(
        AgentWorkerToolSearchRequest {
            input: SearchAgentToolsInput {
                query: None,
                domain: None,
                effect: None,
                capability: None,
                limit: Some(0),
            }
        }
        .validate(),
        Err(WorkerContractError::InvalidValue {
            field: "input.limit",
            ..
        })
    ));

    assert!(matches!(
        AgentWorkerToolDescribeRequest {
            input: DescribeAgentToolInput { name: " ".into() }
        }
        .validate(),
        Err(WorkerContractError::BlankField {
            field: "input.name"
        })
    ));
    let skill_search = AgentWorkerSkillSearchRequest {
        run_id: 7,
        lease: valid_request().lease.clone(),
        input: SearchAgentSkillsInput {
            query: Some("visual".into()),
            limit: Some(5),
        },
    };
    skill_search
        .validate()
        .expect("skill search should be valid");
    assert!(matches!(
        AgentWorkerSkillSearchRequest {
            input: SearchAgentSkillsInput {
                query: None,
                limit: Some(0),
            },
            ..skill_search
        }
        .validate(),
        Err(WorkerContractError::InvalidValue {
            field: "input.limit",
            ..
        })
    ));
    AgentWorkerSkillLoadRequest {
        run_id: 7,
        lease: valid_request().lease,
        input: LoadAgentSkillInput {
            id: "a4n-agent-planning".into(),
        },
    }
    .validate()
    .expect("skill load should be valid");
    AgentWorkerChapterScopeRequest {
        user_id: 1,
        project_id: 2,
        chapter_ids: vec![3],
    }
    .validate()
    .expect("chapter scope should be valid");
}

#[test]
fn inbox_requests_validate_claim_identity() {
    let request = AgentWorkerInboxClaimRequest {
        run_id: 7,
        lease: valid_request().lease,
        thread_id: 9,
    };
    request.validate().expect("inbox claim should be valid");
    assert!(matches!(
        AgentWorkerInboxConsumeRequest {
            run_id: 7,
            lease: valid_request().lease,
            inbox_id: 10,
            claim_token: " ".into(),
        }
        .validate(),
        Err(WorkerContractError::BlankField {
            field: "claim_token"
        })
    ));
}

#[test]
fn history_request_validates_run_lease() {
    let request = AgentWorkerHistoryRequest {
        run_id: 7,
        lease: valid_request().lease,
    };
    request.validate().expect("history request should be valid");
    assert!(matches!(
        AgentWorkerHistoryRequest {
            run_id: 0,
            lease: request.lease,
        }
        .validate(),
        Err(WorkerContractError::InvalidValue {
            field: "run_id",
            reason: "must be positive"
        })
    ));
}

#[test]
fn recovery_request_validates_run_lease_and_reason() {
    let request = AgentWorkerRecoveryRequest {
        run_id: 7,
        lease: valid_request().lease,
        reason: "snapshot missing".into(),
    };
    request
        .validate()
        .expect("recovery request should be valid");
    assert!(matches!(
        AgentWorkerRecoveryRequest {
            reason: "  ".into(),
            ..request
        }
        .validate(),
        Err(WorkerContractError::BlankField { field: "reason" })
    ));
}

#[test]
fn editor_wait_request_validates_run_lease_and_execution() {
    let request = AgentWorkerEditorWaitRequest {
        run_id: 7,
        lease: valid_request().lease,
        editor_request: AgentEditorRequest {
            execution_id: "execution-1".into(),
            source: a4n_api_contract::agent::AgentEditorSource {
                view_instance_id: "view-1".into(),
                project_id: 7,
                chapter_id: 8,
                editor_revision: "revision-1".into(),
                scene_id: None,
                shot_id: None,
                keyframe_id: None,
                entity_id: None,
            },
            operation: a4n_api_contract::agent::AgentEditorOperation::Inspect,
        },
    };
    request
        .validate()
        .expect("editor wait request should be valid");
    assert!(matches!(
        AgentWorkerEditorWaitRequest {
            editor_request: AgentEditorRequest {
                execution_id: " ".into(),
                ..request.editor_request
            },
            ..request
        }
        .validate(),
        Err(WorkerContractError::BlankField {
            field: "editor_request.execution_id"
        })
    ));
}

#[test]
fn editor_wait_expiry_request_requires_positive_limit() {
    AgentWorkerEditorWaitExpiryRequest { limit: 1 }
        .validate()
        .expect("editor wait expiry limit should be valid");
    assert!(matches!(
        AgentWorkerEditorWaitExpiryRequest { limit: 0 }.validate(),
        Err(WorkerContractError::InvalidValue { field: "limit", .. })
    ));
}

#[test]
fn approval_expiry_request_requires_positive_limit() {
    AgentWorkerApprovalExpiryRequest { limit: 1 }
        .validate()
        .expect("approval expiry limit should be valid");
    assert!(matches!(
        AgentWorkerApprovalExpiryRequest { limit: 0 }.validate(),
        Err(WorkerContractError::InvalidValue { field: "limit", .. })
    ));
}

#[test]
fn expired_run_recovery_request_validates_limit_and_message() {
    AgentWorkerExpiredRunRecoveryRequest {
        limit: 1,
        error_message: Some("lease expired".into()),
    }
    .validate()
    .expect("expired run recovery request should be valid");
    assert!(matches!(
        AgentWorkerExpiredRunRecoveryRequest {
            limit: 0,
            error_message: None,
        }
        .validate(),
        Err(WorkerContractError::InvalidValue { field: "limit", .. })
    ));
    assert!(matches!(
        AgentWorkerExpiredRunRecoveryRequest {
            limit: 1,
            error_message: Some(" ".into()),
        }
        .validate(),
        Err(WorkerContractError::BlankField {
            field: "error_message"
        })
    ));
}

#[test]
fn external_task_wait_requests_validate_cursor_and_ids() {
    AgentWorkerExternalTaskWaitScanRequest { after_run_id: 0 }
        .validate()
        .expect("wait scan cursor should be valid");
    assert!(matches!(
        AgentWorkerExternalTaskWaitScanRequest { after_run_id: -1 }.validate(),
        Err(WorkerContractError::InvalidValue {
            field: "after_run_id",
            ..
        })
    ));
    AgentWorkerExternalTaskWaitWakeRequest {
        run_id: 7,
        execution_id: 8,
    }
    .validate()
    .expect("wait wake ids should be valid");
    assert!(matches!(
        AgentWorkerExternalTaskWaitWakeRequest {
            run_id: 0,
            execution_id: 8,
        }
        .validate(),
        Err(WorkerContractError::InvalidValue {
            field: "run_id",
            ..
        })
    ));
}

#[test]
fn message_request_validates_operation_specific_identifiers() {
    let request = AgentWorkerMessageRequest {
        run_id: 7,
        lease: valid_request().lease,
        operation: AgentWorkerMessageOperation::AppendTool {
            execution_id: 2,
            tool_name: "read".into(),
            tool_call_id: Some("call-1".into()),
            result: serde_json::json!({"ok": true}),
        },
    };
    request.validate().expect("message request should be valid");
    let encoded = serde_json::to_string(&request).expect("message request should serialize");
    serde_json::from_str::<AgentWorkerMessageRequest>(&encoded)
        .expect("message request should deserialize");

    let invalid = AgentWorkerMessageRequest {
        operation: AgentWorkerMessageOperation::AppendTool {
            execution_id: 0,
            tool_name: "read".into(),
            tool_call_id: None,
            result: serde_json::json!({}),
        },
        ..request
    };
    assert!(matches!(
        invalid.validate(),
        Err(WorkerContractError::InvalidValue {
            field: "operation.execution_id",
            ..
        })
    ));
}

#[test]
fn budget_request_requires_a_valid_lease() {
    let request = AgentWorkerBudgetRequest {
        run_id: 7,
        lease: valid_request().lease,
    };
    request.validate().expect("budget request should be valid");
    let invalid = AgentWorkerBudgetRequest {
        run_id: 0,
        ..request
    };
    assert!(matches!(
        invalid.validate(),
        Err(WorkerContractError::InvalidValue {
            field: "run_id",
            ..
        })
    ));
}

#[test]
fn run_payload_requires_a_consistent_worker_snapshot() {
    let timestamp = chrono::NaiveDate::from_ymd_opt(2030, 1, 1)
        .expect("date")
        .and_hms_opt(0, 0, 0)
        .expect("time");
    let payload = AgentRunPayload {
        run_id: 7,
        thread_id: 8,
        turn_id: 9,
        profile_id: "general".into(),
        worker_run: AgentWorkerRunWire {
            run: AgentWorkerRunSnapshotWire {
                id: 7,
                thread_id: 8,
                turn_id: 9,
                parent_run_id: None,
                attempt: 1,
                status: "running".into(),
                budget_json: serde_json::json!({}),
                usage_json: serde_json::json!({}),
                worker_id: Some("worker-1".into()),
                lease_token: Some("fence-1".into()),
                lease_expires_at: Some(timestamp),
                created_at: timestamp,
                started_at: Some(timestamp),
            },
            thread: AgentWorkerThreadSnapshotWire {
                id: 8,
                owner_user_id: 11,
                thread_kind: "main".into(),
                profile_id: "general".into(),
                title: None,
                project_id: Some(12),
                resource_type: None,
                resource_id: None,
                scope_json: serde_json::json!({}),
                default_model_config_id: None,
                status: "active".into(),
                revision: 1,
                context_summary: None,
            },
            lease_token: "fence-1".into(),
        },
        runtime_config_snapshot: None,
    };
    payload
        .validate()
        .expect("worker snapshot should match payload");

    let encoded = serde_json::to_string(&payload).expect("payload should serialize");
    let decoded: AgentRunPayload =
        serde_json::from_str(&encoded).expect("payload should deserialize");
    assert_eq!(decoded, payload);

    let mut inconsistent = payload;
    inconsistent.worker_run.thread.profile_id = "other".into();
    assert!(matches!(
        inconsistent.validate(),
        Err(WorkerContractError::InvalidRelation(_))
    ));
}
