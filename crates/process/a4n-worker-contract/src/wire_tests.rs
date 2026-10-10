use serde::de::DeserializeOwned;

use super::*;

#[test]
fn control_request_lease_has_one_canonical_shape() {
    let lease = WorkerLease::control_request("worker-test");
    assert_eq!(lease.worker_id, "worker-test");
    assert_eq!(lease.epoch, 1);
    assert_eq!(lease.fence_token, "control-request");
    assert_eq!(lease.expires_at, "9999-12-31T23:59:59Z");
    lease
        .validate()
        .expect("control request lease should validate");
}

#[test]
fn worker_command_validation_shares_identity_checks() {
    let request = valid_envelope();
    request
        .validate_worker_command("content.update", "content-worker", "content worker")
        .expect("matching command identity should validate");

    let operation_error = request
        .validate_worker_command("content.read", "content-worker", "content worker")
        .expect_err("operation mismatch should fail");
    assert_eq!(
        operation_error.message(),
        "operation_id 不匹配: expected=content.read, actual=content.update"
    );

    let worker_error = request
        .validate_worker_command("content.update", "other-worker", "other worker")
        .expect_err("worker mismatch should fail");
    assert_eq!(
        worker_error.message(),
        "lease.worker_id 不属于 other worker"
    );

    let mut invalid_schema = request;
    invalid_schema.schema_version = 0;
    let schema_error = invalid_schema
        .validate_worker_command("content.update", "content-worker", "content worker")
        .expect_err("invalid schema should fail before command identity checks");
    let schema_error: WorkerError = schema_error.into();
    assert_eq!(schema_error.code, WorkerErrorCode::InvalidRequest);
    assert!(!schema_error.retryable);
    assert_eq!(
        schema_error.message,
        format!("unsupported worker schema version: expected {WORKER_SCHEMA_VERSION}, got 0")
    );
}

#[test]
fn worker_project_scope_requires_positive_payload_matching_envelope() {
    let request = valid_envelope();

    assert_eq!(request.require_project_scope(7).expect("scope"), 7);
    for project_id in [0, -1, 8] {
        let error = request
            .require_project_scope(project_id)
            .expect_err("invalid or mismatched project ID should fail");
        assert_eq!(
            error.message(),
            "worker scope project_id 与 payload 不一致或无效"
        );
    }

    let mut unscoped = request;
    unscoped.project_id = None;
    unscoped.root_project_id = None;
    let error = unscoped
        .require_project_scope(7)
        .expect_err("required project scope should not be absent");
    assert_eq!(
        error.message(),
        "worker scope project_id 与 payload 不一致或无效"
    );
}

#[test]
fn worker_optional_project_scope_allows_absent_scope_and_rejects_invalid_resource() {
    assert!(validate_optional_project_scope(None, 7).is_ok());
    assert!(validate_optional_project_scope(Some(7), 7).is_ok());

    for (scope, project_id) in [(Some(8), 7), (Some(7), 0), (None, -1)] {
        let error = validate_optional_project_scope(scope, project_id)
            .expect_err("invalid or mismatched resource scope should fail");
        assert_eq!(
            error.message(),
            "worker scope project_id 与资源 project_id 不一致或无效"
        );
    }
}

#[test]
fn worker_failure_constructors_share_wire_error_semantics() {
    let cases = [
        (
            WorkerFailure::invalid_request("bad request"),
            WorkerErrorCode::InvalidRequest,
            false,
            "bad request",
        ),
        (
            WorkerFailure::unauthorized("forbidden"),
            WorkerErrorCode::Unauthorized,
            false,
            "forbidden",
        ),
        (
            WorkerFailure::not_found("missing"),
            WorkerErrorCode::NotFound,
            false,
            "missing",
        ),
        (
            WorkerFailure::conflict("conflict"),
            WorkerErrorCode::Conflict,
            false,
            "conflict",
        ),
        (
            WorkerFailure::unavailable("database unavailable"),
            WorkerErrorCode::Unavailable,
            true,
            "database unavailable",
        ),
        (
            WorkerFailure::internal("internal"),
            WorkerErrorCode::Internal,
            false,
            "internal",
        ),
    ];

    for (failure, code, retryable, message) in cases {
        let error: WorkerError = failure.into();
        assert_eq!(error.code, code);
        assert_eq!(error.retryable, retryable);
        assert_eq!(error.message, message);
        error.validate().expect("failure message validates");
    }
}

#[test]
fn worker_positive_id_validation_shares_invalid_request_semantics() {
    assert_eq!(
        WorkerFailure::require_positive_id(7, "project_id").expect("positive ID"),
        7
    );
    for value in [0, -1] {
        let error: WorkerError = WorkerFailure::require_positive_id(value, "project_id")
            .expect_err("non-positive ID should fail")
            .into();
        assert_eq!(error.code, WorkerErrorCode::InvalidRequest);
        assert!(!error.retryable);
        assert_eq!(error.message, "project_id 必须大于 0");
    }
}

#[test]
fn worker_non_blank_validation_preserves_trimmed_value_and_error() {
    assert_eq!(
        WorkerFailure::require_non_blank("  value  ", "field 不能为空").expect("nonblank"),
        "value"
    );
    for value in ["", "  ", "\t\n"] {
        let error: WorkerError = WorkerFailure::require_non_blank(value, "field 不能为空")
            .expect_err("blank value should fail")
            .into();
        assert_eq!(error.code, WorkerErrorCode::InvalidRequest);
        assert!(!error.retryable);
        assert_eq!(error.message, "field 不能为空");
    }
}

#[test]
fn worker_principal_user_id_validation_preserves_kind_and_parse_boundaries() {
    let user = WorkerPrincipal {
        kind: WorkerPrincipalKind::User,
        subject: "42".into(),
    };
    assert_eq!(user.require_user_id().expect("user ID"), 42);
    assert_eq!(user.parse_user_id().expect("parsed ID"), 42);

    let service = WorkerPrincipal {
        kind: WorkerPrincipalKind::Service,
        subject: "42".into(),
    };
    assert_eq!(service.parse_user_id().expect("numeric subject"), 42);
    let error: WorkerError = service
        .require_user_id()
        .expect_err("service is not a user")
        .into();
    assert_eq!(error.code, WorkerErrorCode::Unauthorized);
    assert_eq!(error.message, "仅允许用户 principal");

    let zero = WorkerPrincipal {
        kind: WorkerPrincipalKind::User,
        subject: "0".into(),
    };
    let error: WorkerError = zero.parse_user_id().expect_err("zero user ID").into();
    assert_eq!(error.code, WorkerErrorCode::Unauthorized);
    assert_eq!(error.message, "用户 principal subject 无效");
}

#[test]
fn worker_principal_kind_validation_preserves_custom_unauthorized_message() {
    let service = WorkerPrincipal {
        kind: WorkerPrincipalKind::Service,
        subject: "runtime".into(),
    };
    service
        .require_kind(WorkerPrincipalKind::Service, "service required")
        .expect("matching principal kind");
    let error: WorkerError = service
        .require_kind(WorkerPrincipalKind::User, "user required")
        .expect_err("mismatched principal kind")
        .into();
    assert_eq!(error.code, WorkerErrorCode::Unauthorized);
    assert!(!error.retryable);
    assert_eq!(error.message, "user required");
}

#[test]
fn worker_responses_share_canonical_command_metadata() {
    let request = valid_envelope();
    let completed = WorkerResponse::completed_for(&request, serde_json::json!({"ok": true}));
    assert_eq!(completed.schema_version, WORKER_SCHEMA_VERSION);
    assert_eq!(completed.request_id, request.request_id);
    assert_eq!(completed.trace_id, request.trace_id);
    assert_eq!(completed.operation.operation_id, request.operation_id);
    assert_eq!(completed.operation.task_id, None);
    assert_eq!(completed.status, WorkerOperationStatus::Completed);
    assert_eq!(completed.payload, Some(serde_json::json!({"ok": true})));
    completed.validate().expect("completed response validates");

    let failed = WorkerResponse::<serde_json::Value>::failed_for(
        &request,
        WorkerFailure::unavailable("database unavailable").into(),
    );
    assert_eq!(failed.request_id, request.request_id);
    assert_eq!(failed.trace_id, request.trace_id);
    assert_eq!(failed.operation.operation_id, request.operation_id);
    assert_eq!(failed.operation.task_id, None);
    assert_eq!(failed.status, WorkerOperationStatus::Failed);
    assert_eq!(failed.payload, None);
    assert_eq!(
        failed.error.as_ref().map(|error| error.code),
        Some(WorkerErrorCode::Unavailable)
    );
    failed.validate().expect("failed response validates");

    let failed_from_parts = WorkerResponse::<serde_json::Value>::failed_for_parts(
        "request-2".into(),
        "trace-2".into(),
        "content.update".into(),
        WorkerFailure::internal("conversion failed").into(),
    );
    assert_eq!(failed_from_parts.request_id, "request-2");
    assert_eq!(failed_from_parts.trace_id, "trace-2");
    assert_eq!(failed_from_parts.operation.operation_id, "content.update");
    assert_eq!(failed_from_parts.operation.task_id, None);
    assert_eq!(failed_from_parts.status, WorkerOperationStatus::Failed);
    assert_eq!(failed_from_parts.payload, None);
    assert_eq!(
        failed_from_parts.error.as_ref().map(|error| error.code),
        Some(WorkerErrorCode::Internal)
    );
    failed_from_parts
        .validate()
        .expect("failed response from parts validates");
}

fn valid_envelope() -> WorkerEnvelope<serde_json::Value> {
    WorkerEnvelope {
        schema_version: WORKER_SCHEMA_VERSION,
        request_id: "request-1".into(),
        trace_id: "trace-1".into(),
        principal: WorkerPrincipal {
            kind: WorkerPrincipalKind::Service,
            subject: "control".into(),
        },
        operation_id: "content.update".into(),
        idempotency_key: "idem-1".into(),
        project_id: Some(7),
        root_project_id: Some(7),
        target_revision: Some(3),
        runtime_config_snapshot_ref: None,
        lease: WorkerLease {
            worker_id: "content-worker".into(),
            epoch: 2,
            fence_token: "fence-2".into(),
            expires_at: "2030-01-01 00:00:00".into(),
        },
        payload: serde_json::json!({"content": "draft"}),
    }
}

fn valid_operation() -> WorkerOperationRef {
    WorkerOperationRef {
        operation_id: "content.update".into(),
        task_id: Some("task-1".into()),
    }
}

fn valid_response(status: WorkerOperationStatus) -> WorkerResponse<serde_json::Value> {
    WorkerResponse {
        schema_version: WORKER_SCHEMA_VERSION,
        request_id: "request-1".into(),
        trace_id: "trace-1".into(),
        operation: valid_operation(),
        status,
        payload: (status == WorkerOperationStatus::Completed)
            .then(|| serde_json::json!({"ok": true})),
        resource_refs: vec![WorkerResourceRef {
            kind: "chapter".into(),
            id: "7".into(),
            revision: Some(WorkerRevision::new(3).expect("revision")),
        }],
        committed_revision: Some(WorkerRevision::new(3).expect("revision")),
        provider_job_id: None,
        result_summary: Some("updated".into()),
        pagination: None,
        error: (status == WorkerOperationStatus::Failed).then(|| WorkerError {
            code: WorkerErrorCode::Internal,
            message: "failed".into(),
            retryable: true,
        }),
    }
}

fn assert_unknown_field_rejected<T: DeserializeOwned + std::fmt::Debug>(
    mut value: serde_json::Value,
) {
    value["future_field"] = serde_json::json!(true);
    let error = serde_json::from_value::<T>(value)
        .expect_err("common worker contract must reject unknown fields");
    assert!(error.to_string().contains("unknown field `future_field`"));
}

#[test]
fn agent_envelope_preserves_wire_shape_and_common_metadata() {
    let request: WorkerEnvelope<_> = valid_envelope();
    request.validate().expect("envelope should validate");
    let encoded = serde_json::to_value(&request).expect("request should serialize");
    assert!(encoded.get("scope").is_none());
    assert_eq!(encoded["idempotency_key"], "idem-1");
    assert_eq!(encoded["target_revision"], 3);
    assert_eq!(request.scope().project_id, Some(7));
    assert_eq!(request.operation_ref().operation_id, "content.update");
    assert_eq!(request.operation_ref().task_id, None);

    let decoded: WorkerEnvelope<serde_json::Value> =
        serde_json::from_value(encoded).expect("agent envelope should deserialize");
    assert_eq!(decoded, request);
}

#[test]
fn envelope_rejects_blank_idempotency_principal_and_invalid_scope_revision() {
    let mut request = valid_envelope();
    request.idempotency_key = " ".into();
    assert!(matches!(
        request.validate(),
        Err(WorkerContractError::BlankField {
            field: "idempotency_key"
        })
    ));

    let mut request = valid_envelope();
    request.principal.subject = " ".into();
    assert!(matches!(
        request.validate(),
        Err(WorkerContractError::BlankField {
            field: "principal.subject"
        })
    ));

    let mut request = valid_envelope();
    request.root_project_id = Some(8);
    request.project_id = None;
    assert_eq!(
        request.validate(),
        Err(WorkerContractError::InvalidRelation(
            "root_project_id requires project_id"
        ))
    );

    let mut request = valid_envelope();
    request.target_revision = Some(0);
    assert!(matches!(
        request.validate(),
        Err(WorkerContractError::InvalidValue {
            field: "target_revision",
            ..
        })
    ));

    let invalid_scope = WorkerScope {
        project_id: Some(7),
        root_project_id: None,
        resource_type: Some("chapter".into()),
        resource_id: None,
    };
    assert!(matches!(
        invalid_scope.validate(),
        Err(WorkerContractError::InvalidRelation(_))
    ));
}

#[test]
fn common_envelopes_reject_unknown_fields() {
    assert_unknown_field_rejected::<WorkerEnvelope<serde_json::Value>>(
        serde_json::to_value(valid_envelope()).expect("envelope should serialize"),
    );
    assert_unknown_field_rejected::<WorkerResponse<serde_json::Value>>(
        serde_json::to_value(valid_response(WorkerOperationStatus::Completed))
            .expect("response should serialize"),
    );
    assert_unknown_field_rejected::<WorkerCancelRequest>(serde_json::json!({
        "operation": {"operation_id": "content.update"},
        "reason": "user requested"
    }));
    assert_unknown_field_rejected::<WorkerRecoveryRequest>(serde_json::json!({
        "operation": {"operation_id": "content.update"},
        "reason": "worker restarted"
    }));
    assert_unknown_field_rejected::<WorkerHealth>(serde_json::json!({
        "schema_version": WORKER_SCHEMA_VERSION,
        "worker_id": "content-worker",
        "status": "healthy",
        "version": "2026.09.17"
    }));
    assert_unknown_field_rejected::<WorkerCapabilities>(serde_json::json!({
        "schema_version": WORKER_SCHEMA_VERSION,
        "worker_id": "content-worker",
        "capabilities": []
    }));
}

#[test]
fn operation_cancel_and_recovery_contracts_validate_identity_and_reason() {
    let cancel = WorkerCancelRequest {
        operation: valid_operation(),
        reason: "user requested".into(),
    };
    cancel.validate().expect("cancel request should validate");
    assert!(matches!(
        WorkerCancelRequest {
            reason: " ".into(),
            ..cancel
        }
        .validate(),
        Err(WorkerContractError::BlankField { field: "reason" })
    ));

    let recovery = WorkerRecoveryRequest {
        operation: valid_operation(),
        reason: "worker restarted".into(),
        checkpoint_ref: Some("checkpoint-1".into()),
    };
    recovery
        .validate()
        .expect("recovery request should validate");
    assert!(matches!(
        WorkerRecoveryRequest {
            checkpoint_ref: Some(" ".into()),
            ..recovery
        }
        .validate(),
        Err(WorkerContractError::BlankField {
            field: "checkpoint_ref"
        })
    ));
}

#[test]
fn response_requires_status_consistent_error_payload_and_pagination() {
    valid_response(WorkerOperationStatus::Accepted)
        .validate()
        .expect("accepted response should validate");
    valid_response(WorkerOperationStatus::Completed)
        .validate()
        .expect("completed response should validate");
    valid_response(WorkerOperationStatus::Failed)
        .validate()
        .expect("failed response should validate");

    let mut failed = valid_response(WorkerOperationStatus::Failed);
    failed.error = None;
    assert_eq!(
        failed.validate(),
        Err(WorkerContractError::InvalidRelation(
            "failed response requires error"
        ))
    );

    let mut running = valid_response(WorkerOperationStatus::Running);
    running.payload = Some(serde_json::json!({"progress": 1}));
    assert_eq!(
        running.validate(),
        Err(WorkerContractError::InvalidRelation(
            "non-completed response cannot include payload"
        ))
    );

    let mut response = valid_response(WorkerOperationStatus::Completed);
    response.pagination = Some(WorkerPagination {
        next_cursor: Some("cursor-2".into()),
        has_more: true,
    });
    response.validate().expect("paged response should validate");
    response.pagination = Some(WorkerPagination {
        next_cursor: None,
        has_more: true,
    });
    assert!(matches!(
        response.validate(),
        Err(WorkerContractError::InvalidRelation(_))
    ));
}

#[test]
fn response_distinguishes_null_results_from_missing_payloads() {
    let mut wire = serde_json::to_value(valid_response(WorkerOperationStatus::Completed))
        .expect("response should serialize");
    wire["payload"] = serde_json::Value::Null;
    let nullable: WorkerResponse<Option<String>> =
        serde_json::from_value(wire.clone()).expect("null business result");
    assert_eq!(nullable.payload, Some(None));
    assert_eq!(serde_json::to_value(&nullable).unwrap(), wire);
    let unit: WorkerResponse<()> = serde_json::from_value(wire.clone()).expect("void result");
    assert_eq!(unit.payload, Some(()));
    assert!(serde_json::from_value::<WorkerResponse<String>>(wire.clone()).is_err());

    wire["payload"] = serde_json::json!("saved");
    let value: WorkerResponse<Option<String>> =
        serde_json::from_value(wire.clone()).expect("non-null result");
    assert_eq!(value.payload, Some(Some("saved".into())));

    wire.as_object_mut().unwrap().remove("payload");
    let missing: WorkerResponse<Option<String>> =
        serde_json::from_value(wire).expect("absent result stays absent");
    assert_eq!(missing.payload, None);
}

#[test]
fn revision_and_response_round_trip_with_zero_rejected() {
    let revision = WorkerRevision::new(4).expect("positive revision should validate");
    let encoded = serde_json::to_string(&revision).expect("revision should serialize");
    assert_eq!(encoded, "4");
    let decoded: WorkerRevision = serde_json::from_str(&encoded).expect("revision");
    assert_eq!(decoded, revision);
    assert!(serde_json::from_str::<WorkerRevision>("0").is_err());

    let response = valid_response(WorkerOperationStatus::Completed);
    let encoded = serde_json::to_string(&response).expect("response should serialize");
    let decoded: WorkerResponse<serde_json::Value> =
        serde_json::from_str(&encoded).expect("response should deserialize");
    assert_eq!(decoded, response);
}

#[test]
fn health_and_capabilities_validate_schema_identity_and_duplicates() {
    let health = WorkerHealth {
        schema_version: WORKER_SCHEMA_VERSION,
        worker_id: "content-worker".into(),
        status: WorkerHealthStatus::Healthy,
        version: "2026.09.17".into(),
    };
    health
        .validate_for("content-worker")
        .expect("health should validate");
    assert!(matches!(
        health.validate_for("other-worker"),
        Err(WorkerContractError::InvalidRelation(_))
    ));

    let capabilities = WorkerCapabilities {
        schema_version: WORKER_SCHEMA_VERSION,
        worker_id: "content-worker".into(),
        capabilities: vec![WorkerCapability {
            name: "content.read".into(),
            version: "v1".into(),
            enabled: true,
        }],
    };
    capabilities
        .validate_for("content-worker")
        .expect("capabilities should validate");

    let duplicate = WorkerCapabilities {
        capabilities: vec![
            WorkerCapability {
                name: "content.read".into(),
                version: "v1".into(),
                enabled: true,
            },
            WorkerCapability {
                name: "content.read".into(),
                version: "v2".into(),
                enabled: true,
            },
        ],
        ..capabilities
    };
    assert!(matches!(
        duplicate.validate(),
        Err(WorkerContractError::InvalidRelation(_))
    ));
}
