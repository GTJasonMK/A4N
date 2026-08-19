use serde_json::json;

use super::requests::service_request_with_context;
use super::*;

#[test]
fn provider_secret_detection_is_recursive_and_key_aware() {
    assert!(payload_has_provider_secret(&json!({
        "provider": {
            "api_key": "secret-key"
        }
    })));
    assert!(!payload_has_provider_secret(&json!({
        "provider": {
            "api_key": " "
        }
    })));
    assert!(!payload_has_provider_secret(&json!({
        "content": "api_key=literal text"
    })));
}

#[test]
fn explicit_context_uses_protocol_only_schema_and_redacts_fields() {
    let context = SidecarRequestContext::current_global()
        .with_user_id(Some(7))
        .with_project_scope(Some(11), Some(3))
        .with_trace_context(TraceContext {
            trace_id: "trace-1".to_string(),
            parent_span_id: Some("parent-1".to_string()),
        })
        .with_runtime_config_snapshot(Some(json!({
            "visible": "keep",
            "api_key_encrypted": "secret-key",
            "nested": {
                "base_url": "https://provider.example"
            }
        })))
        .with_secret_policy(SecretForwardingPolicy::deny_all());

    let request = service_request_with_context(
        context,
        a4n_service_protocol::IMPORT_TRANSFORM_CAPABILITY_VERSION,
        json!({ "x": 1 }),
    )
    .expect("service request should be created");

    assert!(!request.request_id.is_empty());
    assert_eq!(request.trace_context.trace_id, "trace-1");
    assert_eq!(
        request.trace_context.parent_span_id.as_deref(),
        Some("parent-1")
    );
    assert_eq!(
        request.schema_version.protocol,
        a4n_service_protocol::PROTOCOL_VERSION
    );
    assert!(request.schema_version.is_protocol_only());
    assert_eq!(request.user_id, Some(7));
    assert_eq!(request.project_id, Some(11));
    assert_eq!(request.root_project_id, Some(3));
    assert_eq!(request.secret_policy, SecretForwardingPolicy::deny_all());
    assert_eq!(
        request.capability_version,
        a4n_service_protocol::IMPORT_TRANSFORM_CAPABILITY_VERSION
    );
    assert_eq!(request.payload, json!({ "x": 1 }));

    let snapshot = request
        .runtime_config_snapshot
        .expect("runtime config snapshot should be set");
    assert_eq!(snapshot["visible"], "keep");
    assert_eq!(snapshot["api_key_encrypted"], "[REDACTED]");
    assert_eq!(snapshot["nested"]["base_url"], "[REDACTED]");
    let snapshot_text = snapshot.to_string();
    assert!(!snapshot_text.contains("secret-key"));
    assert!(!snapshot_text.contains("provider.example"));
}

#[test]
fn request_builder_rejects_invalid_identity_and_project_scope() {
    let error = service_request_with_context(
        SidecarRequestContext::current_global().with_user_id(Some(0)),
        a4n_service_protocol::IMPORT_TRANSFORM_CAPABILITY_VERSION,
        json!({}),
    )
    .expect_err("non-positive user ID must fail before transport");
    assert!(error.contains("user_id"));
    assert!(error.contains("positive integer"));

    let error = service_request_with_context(
        SidecarRequestContext::current_global().with_project_scope(None, Some(3)),
        a4n_service_protocol::IMPORT_TRANSFORM_CAPABILITY_VERSION,
        json!({}),
    )
    .expect_err("root project without project must fail before transport");
    assert!(error.contains("root_project_id requires project_id"));

    let error = service_request_with_context(
        SidecarRequestContext::current_global().with_trace_context(TraceContext {
            trace_id: " ".to_string(),
            parent_span_id: None,
        }),
        a4n_service_protocol::IMPORT_TRANSFORM_CAPABILITY_VERSION,
        json!({}),
    )
    .expect_err("blank trace ID must fail before transport");
    assert!(error.contains("trace_context.trace_id"));
}

#[tokio::test]
async fn pure_transform_does_not_inherit_task_local_user() {
    runtime_config::request_scope(7, None, async {
        let request = pure_transform_typed_service_request(
            a4n_service_protocol::IMPORT_TRANSFORM_CAPABILITY_VERSION,
            json!({ "project_id": 11 }),
            Some(11),
            Some(3),
        )
        .expect("pure transform request should be created");

        assert_eq!(request.user_id, None);
        assert_eq!(request.project_id, Some(11));
        assert_eq!(request.root_project_id, Some(3));
        assert_eq!(request.runtime_config_snapshot, None);
        assert_eq!(request.secret_policy, SecretForwardingPolicy::deny_all());
    })
    .await;
}

#[test]
fn global_no_secret_request_rejects_top_level_project_scope() {
    let error = global_service_request_without_secrets(
        a4n_service_protocol::IMPORT_TRANSFORM_CAPABILITY_VERSION,
        json!({
            "project_id": 21,
            "projectId": "21",
            "root_project_id": 9,
            "rootProjectId": "9"
        }),
    )
    .expect_err("global request must reject project scope hidden in payload");

    assert!(error.contains("global sidecar request 不能携带项目作用域"));
}

#[test]
fn null_payload_project_scope_is_treated_as_absent() {
    let request = global_service_request_without_secrets(
        a4n_service_protocol::IMPORT_TRANSFORM_CAPABILITY_VERSION,
        json!({ "project_id": null, "rootProjectId": null }),
    )
    .expect("optional null project scope should remain absent");

    assert_eq!(request.project_id, None);
    assert_eq!(request.root_project_id, None);
}

#[test]
fn payload_project_scope_rejects_invalid_or_conflicting_aliases() {
    for payload in [
        json!({ "project_id": "not-an-id" }),
        json!({ "project_id": 0 }),
        json!({ "project_id": 21, "projectId": 22 }),
        json!({ "root_project_id": 9, "rootProjectId": 10 }),
    ] {
        let error = global_service_request_without_secrets(
            a4n_service_protocol::IMPORT_TRANSFORM_CAPABILITY_VERSION,
            payload,
        )
        .expect_err("invalid or conflicting project scope aliases must fail");
        assert!(
            error.contains("必须是正整数") || error.contains("别名字段不一致"),
            "unexpected error: {error}"
        );
    }
}

#[test]
fn explicit_context_must_match_payload_project_scope() {
    let request = service_request_with_context(
        SidecarRequestContext::current_global().with_project_scope(Some(21), Some(9)),
        a4n_service_protocol::IMPORT_TRANSFORM_CAPABILITY_VERSION,
        json!({ "project_id": 21, "root_project_id": 9 }),
    )
    .expect("matching context and payload scope should pass");
    assert_eq!(request.project_id, Some(21));
    assert_eq!(request.root_project_id, Some(9));

    for payload in [
        json!({ "project_id": 22, "root_project_id": 9 }),
        json!({ "project_id": 21, "root_project_id": 10 }),
    ] {
        let error = service_request_with_context(
            SidecarRequestContext::current_global().with_project_scope(Some(21), Some(9)),
            a4n_service_protocol::IMPORT_TRANSFORM_CAPABILITY_VERSION,
            payload,
        )
        .expect_err("context and payload scope mismatch must fail");
        assert!(error.contains("context."));
        assert!(error.contains("与 payload."));
        assert!(error.contains("不一致"));
    }
}

#[test]
fn current_context_denies_secret_forwarding_by_default() {
    let request = service_request_with_context(
        SidecarRequestContext::current_global(),
        a4n_service_protocol::IMPORT_TRANSFORM_CAPABILITY_VERSION,
        json!({ "api_key": "secret" }),
    )
    .expect("service request should be created");

    assert_eq!(request.secret_policy, SecretForwardingPolicy::deny_all());
}

#[test]
fn provider_secret_policy_allows_only_loopback_and_preserves_scope() {
    let request = service_request_with_context_provider_secret_policy_for_url(
        SidecarRequestContext::current_global().with_project_scope(Some(21), Some(9)),
        "http://localhost:47645",
        a4n_service_protocol::IMAGE_PROVIDER_CAPABILITY_VERSION,
        json!({ "api_key": "secret" }),
    )
    .expect("loopback sidecar should accept local user secrets");

    assert_eq!(request.project_id, Some(21));
    assert_eq!(request.root_project_id, Some(9));
    assert_eq!(
        request.secret_policy,
        SecretForwardingPolicy::local_user_secret()
    );

    let error = global_provider_service_request_for_url(
        "https://sidecar.example.com",
        a4n_service_protocol::IMAGE_PROVIDER_CAPABILITY_VERSION,
        json!({ "api_key": "secret" }),
    )
    .expect_err("remote sidecar must not receive provider secrets");
    assert!(error.contains("拒绝转发 provider secret"));
    assert!(!error.contains("sidecar.example.com"));

    let remote_without_secret = global_provider_service_request_for_url(
        "https://sidecar.example.com",
        a4n_service_protocol::IMAGE_PROVIDER_CAPABILITY_VERSION,
        json!({ "api_key": null, "prompt": "content" }),
    )
    .expect("remote content without provider secret should use explicit egress scope");
    assert_eq!(
        remote_without_secret.secret_policy,
        SecretForwardingPolicy::trusted_remote_no_secrets()
    );
}

#[test]
fn global_provider_request_rejects_project_scope_hidden_in_payload() {
    let error = global_provider_service_request_for_url(
        "http://localhost:47645",
        a4n_service_protocol::IMAGE_PROVIDER_CAPABILITY_VERSION,
        json!({ "project_id": 21, "api_key": "secret" }),
    )
    .expect_err("global provider request must reject project scope");

    assert!(error.contains("global sidecar request 不能携带项目作用域"));
}

#[test]
fn provider_request_without_secret_uses_trusted_remote_without_secrets() {
    let request = global_provider_service_request_for_url(
        "https://sidecar.example.com",
        a4n_service_protocol::LLM_CHAT_CAPABILITY_VERSION,
        json!({ "api_key": null }),
    )
    .expect("remote sidecar request without secret should be allowed");

    assert_eq!(
        request.secret_policy,
        SecretForwardingPolicy::trusted_remote_no_secrets()
    );
}

#[test]
fn llm_trace_prefers_generation_then_workflow_run() {
    let generation_trace = llm_trace(Some(42), Some(77), Some("draft call"));
    let request = service_request_with_context(
        sidecar_context_for_llm_trace(&generation_trace)
            .expect("positive generation/workflow trace IDs"),
        a4n_service_protocol::LLM_CHAT_CAPABILITY_VERSION,
        json!({ "messages": [] }),
    )
    .expect("service request should be created");
    assert_eq!(request.project_id, Some(11));
    assert_eq!(request.root_project_id, Some(3));
    assert_eq!(request.trace_context.trace_id, "generation_run:42");
    assert_eq!(
        request.trace_context.parent_span_id.as_deref(),
        Some("chapter_generation:draft.step:draft_call")
    );

    let workflow_trace = llm_trace(None, Some(77), None);
    let workflow_request = service_request_with_context(
        sidecar_context_for_llm_trace(&workflow_trace).expect("positive workflow trace ID"),
        a4n_service_protocol::LLM_CHAT_CAPABILITY_VERSION,
        json!({ "messages": [] }),
    )
    .expect("service request should be created");
    assert_eq!(workflow_request.trace_context.trace_id, "workflow_run:77");
}

#[test]
fn llm_trace_normalizes_components_and_rejects_blank_step() {
    let mut trace = LlmTraceContext {
        project_id: Some(11),
        root_project_id: None,
        workflow: " ad hoc/chat ".to_string(),
        step_key: " draft step ".to_string(),
        generation_run_id: None,
        workflow_run_id: None,
        call_key: Some("manual#1".to_string()),
        input_fingerprint_sha256: None,
        context_plan_json: None,
        context_pack_refs_json: None,
    };
    let request = service_request_with_context(
        sidecar_context_for_llm_trace(&trace).expect("ad hoc LLM trace without run ID"),
        a4n_service_protocol::LLM_CHAT_CAPABILITY_VERSION,
        json!({ "messages": [] }),
    )
    .expect("service request should be created");

    assert_eq!(
        request.trace_context.trace_id,
        "llm:project:11:ad_hoc_chat:draft_step:manual_1"
    );
    assert_eq!(
        request.trace_context.parent_span_id.as_deref(),
        Some("ad_hoc_chat:draft_step:manual_1")
    );

    trace.step_key = " ".to_string();
    let error = sidecar_context_for_llm_trace(&trace)
        .expect_err("blank LLM trace step must fail before request construction");
    assert!(error.contains("step_key 不能为空"));
}

#[test]
fn provider_secret_policy_preserves_explicit_llm_trace() {
    let trace = llm_trace(Some(42), None, Some("draft call"));
    let request = service_request_with_context_provider_secret_policy_for_url(
        sidecar_context_for_llm_trace(&trace).expect("positive generation trace ID"),
        "http://localhost:47642",
        a4n_service_protocol::LLM_CHAT_CAPABILITY_VERSION,
        json!({ "provider": { "api_key": "secret" }, "project_id": 11 }),
    )
    .expect("loopback sidecar should accept provider secrets");
    assert_eq!(request.project_id, Some(11));
    assert_eq!(request.root_project_id, Some(3));
    assert_eq!(request.trace_context.trace_id, "generation_run:42");
    assert_eq!(
        request.secret_policy,
        SecretForwardingPolicy::local_user_secret()
    );

    let error = service_request_with_context_provider_secret_policy_for_url(
        sidecar_context_for_llm_trace(&trace).expect("positive generation trace ID"),
        "http://localhost:47642",
        a4n_service_protocol::LLM_CHAT_CAPABILITY_VERSION,
        json!({ "provider": { "api_key": "secret" }, "project_id": 99 }),
    )
    .expect_err("provider payload must not disagree with authoritative trace scope");
    assert!(error.contains("context.project_id"));
    assert!(error.contains("payload.project_id"));

    let error = service_request_with_context_provider_secret_policy_for_url(
        sidecar_context_for_llm_trace(&trace).expect("positive generation trace ID"),
        "https://sidecar.example.com",
        a4n_service_protocol::LLM_CHAT_CAPABILITY_VERSION,
        json!({ "provider": { "api_key": "secret" } }),
    )
    .expect_err("remote sidecar must not receive provider secrets");
    assert!(error.contains("拒绝转发 provider secret"));
}

#[test]
fn llm_trace_rejects_non_positive_generation_or_workflow_run_id() {
    for (field, trace) in [
        ("generation_run_id", llm_trace(Some(0), Some(77), None)),
        ("generation_run_id", llm_trace(Some(-1), None, None)),
        ("workflow_run_id", llm_trace(None, Some(0), None)),
        ("workflow_run_id", llm_trace(None, Some(-1), None)),
        ("workflow_run_id", llm_trace(Some(42), Some(0), None)),
        ("workflow_run_id", llm_trace(Some(42), Some(-1), None)),
    ] {
        let error = sidecar_context_for_llm_trace(&trace)
            .expect_err("non-positive LLM trace run IDs must fail before request construction");
        assert!(error.contains(field));
        assert!(error.contains("必须是正整数"));
    }
}

#[test]
fn workflow_operation_context_preserves_root_and_rejects_invalid_run_id() {
    let context = sidecar_context_for_workflow_operation_with_root(
        Some(11),
        Some(3),
        Some(77),
        "image.generate",
    )
    .expect("root-scoped workflow context");
    let request = service_request_with_context(
        context,
        a4n_service_protocol::IMAGE_PROVIDER_CAPABILITY_VERSION,
        json!({ "project_id": 11 }),
    )
    .expect("root-scoped workflow request");

    assert_eq!(request.project_id, Some(11));
    assert_eq!(request.root_project_id, Some(3));
    assert_eq!(request.trace_context.trace_id, "workflow_run:77");
    assert_eq!(
        request.trace_context.parent_span_id.as_deref(),
        Some("image.generate")
    );

    let error = sidecar_context_for_workflow_operation_with_root(
        Some(11),
        Some(3),
        Some(0),
        "image.generate",
    )
    .expect_err("invalid workflow run must fail");
    assert!(error.contains("workflow_run_id 必须是正整数"));
}

fn llm_trace(
    generation_run_id: Option<i64>,
    workflow_run_id: Option<i64>,
    call_key: Option<&str>,
) -> LlmTraceContext {
    LlmTraceContext {
        project_id: Some(11),
        root_project_id: Some(3),
        workflow: "chapter generation".to_string(),
        step_key: "draft.step".to_string(),
        generation_run_id,
        workflow_run_id,
        call_key: call_key.map(str::to_string),
        input_fingerprint_sha256: None,
        context_plan_json: None,
        context_pack_refs_json: None,
    }
}
