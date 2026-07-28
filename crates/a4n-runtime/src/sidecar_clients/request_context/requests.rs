use a4n_capabilities::JsonRequest;
use a4n_service_protocol::{
    SecretForwardingPolicy, SecretMaterialScope, ServiceRequest, TraceContext,
    egress_data_type_for_service, service_for_capability_version, validate_egress_scope,
    validate_service_request_context,
};
use serde::Serialize;
use serde_json::Value;

use super::{
    context::SidecarRequestContext,
    scope::{merge_project_scope, payload_project_scope},
    secrets::sidecar_url_allows_local_secrets,
    trace::request_id,
};
use crate::sidecar_clients::service_manager::sidecar_url_for_service;

pub(crate) fn global_service_request_without_secrets(
    capability_version: impl Into<String>,
    payload: impl Serialize,
) -> Result<JsonRequest, String> {
    let payload = serde_json::to_value(payload).map_err(|err| err.to_string())?;
    let request = service_request_value_with_context_and_policy(
        SidecarRequestContext::current_global(),
        capability_version,
        payload,
        SecretForwardingPolicy::deny_all(),
    )?;
    ensure_global_request_scope(&request)?;
    Ok(request)
}

/// 纯 transform sidecar 请求：deny-all secret，只携带显式 project 上下文。
pub(crate) fn pure_transform_service_request(
    capability_version: impl Into<String>,
    payload: impl Serialize,
    project_id: Option<i64>,
    root_project_id: Option<i64>,
) -> Result<JsonRequest, String> {
    let payload = serde_json::to_value(payload).map_err(|err| err.to_string())?;
    let context =
        SidecarRequestContext::stateless().with_project_scope(project_id, root_project_id);
    service_request_value_with_context_and_policy(
        context,
        capability_version,
        payload,
        SecretForwardingPolicy::deny_all(),
    )
}

pub(crate) fn service_request_with_context(
    mut context: SidecarRequestContext,
    capability_version: impl Into<String>,
    payload: impl Serialize,
) -> Result<JsonRequest, String> {
    let payload = serde_json::to_value(payload).map_err(|err| err.to_string())?;
    let (project_id, root_project_id) = payload_project_scope(&payload)?;
    merge_project_scope("project_id", &mut context.project_id, project_id)?;
    merge_project_scope(
        "root_project_id",
        &mut context.root_project_id,
        root_project_id,
    )?;
    let capability_version = capability_version.into();
    let secret_policy = resolve_egress_policy(&capability_version, context.secret_policy)?;
    let request_id = request_id();
    let trace_context = context.trace_context.unwrap_or_else(|| TraceContext {
        trace_id: request_id.clone(),
        parent_span_id: None,
    });
    let request = ServiceRequest {
        request_id,
        trace_context,
        schema_version: context.schema_version,
        user_id: context.user_id,
        project_id: context.project_id,
        root_project_id: context.root_project_id,
        runtime_config_snapshot: context.runtime_config_snapshot,
        secret_policy,
        capability_version,
        payload,
    };
    validate_service_request_context(&request).map_err(|error| error.message)?;
    Ok(request)
}

fn resolve_egress_policy(
    capability_version: &str,
    policy: SecretForwardingPolicy,
) -> Result<SecretForwardingPolicy, String> {
    let Some(service) = service_for_capability_version(capability_version) else {
        return Ok(policy);
    };
    let policy = if policy.is_deny_all() {
        let url = sidecar_url_for_service(service)?;
        if sidecar_url_allows_local_secrets(&url) {
            policy
        } else {
            SecretForwardingPolicy::trusted_remote_no_secrets()
        }
    } else {
        policy
    };
    if matches!(policy.scope, SecretMaterialScope::TrustedRemote) {
        let data_type = egress_data_type_for_service(service)
            .ok_or_else(|| format!("sidecar service {service} 缺少 egress data type"))?;
        let event = validate_egress_scope(service, data_type, &policy.scope)
            .map_err(|error| error.message)?;
        tracing::info!(
            target: "a4n_sidecar_egress_audit",
            service,
            data_type = ?event.data_type,
            audit_event = event.audit_event,
            "authorized trusted-remote sidecar egress"
        );
    }
    Ok(policy)
}

pub(super) fn ensure_global_request_scope(request: &JsonRequest) -> Result<(), String> {
    if request.project_id.is_some() || request.root_project_id.is_some() {
        return Err(format!(
            "global sidecar request 不能携带项目作用域: project_id={:?}, root_project_id={:?}",
            request.project_id, request.root_project_id
        ));
    }
    Ok(())
}

pub(super) fn service_request_value_with_context_and_policy(
    context: SidecarRequestContext,
    capability_version: impl Into<String>,
    payload: Value,
    secret_policy: SecretForwardingPolicy,
) -> Result<JsonRequest, String> {
    service_request_with_context(
        context.with_secret_policy(secret_policy),
        capability_version,
        payload,
    )
}
