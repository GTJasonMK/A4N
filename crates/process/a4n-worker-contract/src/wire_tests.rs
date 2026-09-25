use serde::de::DeserializeOwned;

use super::*;

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
