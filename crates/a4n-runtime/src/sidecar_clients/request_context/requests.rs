use a4n_service_protocol::{
    SecretForwardingPolicy, SecretMaterialScope, ServiceRequest, ServiceRequestContext,
    TraceContext, egress_data_type_for_service, validate_egress_scope,
};
use serde::Serialize;
use serde_json::Value;

use super::{
    context::SidecarRequestContext,
    scope::{merge_project_scope, payload_project_scope},
};
use crate::sidecar_clients::service_manager::sidecar_service_and_url_for_capability;
use a4n_service_manager::is_loopback_http_url;

type JsonRequest = ServiceRequest<Value>;

#[cfg(test)]
pub(crate) fn global_service_request_without_secrets(
    capability_version: impl Into<String>,
    payload: impl Serialize,
) -> Result<JsonRequest, String> {
    let request = service_request_with_context(
        SidecarRequestContext::current_global(),
        capability_version,
        payload,
    )?;
    ensure_global_request_scope(&request)?;
    Ok(request)
}

pub(crate) fn global_typed_service_request_without_secrets<T: Serialize>(
    capability_version: impl Into<String>,
    payload: T,
) -> Result<ServiceRequest<T>, String> {
    let payload_value = serde_json::to_value(&payload).map_err(|error| error.to_string())?;
    let request = service_request_with_value_context(
        SidecarRequestContext::current_global(),
        capability_version,
        payload_value,
    )?;
    ensure_global_request_scope(&request)?;
    Ok(ServiceRequest {
        request_id: request.request_id,
        trace_context: request.trace_context,
        schema_version: request.schema_version,
        user_id: request.user_id,
        project_id: request.project_id,
        root_project_id: request.root_project_id,
        runtime_config_snapshot: request.runtime_config_snapshot,
        secret_policy: request.secret_policy,
        capability_version: request.capability_version,
        payload,
    })
}

pub(crate) fn pure_transform_typed_service_request<T: Serialize>(
    capability_version: impl Into<String>,
    payload: T,
    project_id: Option<i64>,
    root_project_id: Option<i64>,
) -> Result<ServiceRequest<T>, String> {
    let payload_scope = serde_json::to_value(&payload).map_err(|err| err.to_string())?;
    let capability_version = capability_version.into();
    let context = resolved_service_request_context(
        SidecarRequestContext::stateless().with_project_scope(project_id, root_project_id),
        &capability_version,
        &payload_scope,
    )?;
    context
        .with_payload(capability_version, payload)
        .map_err(|error| error.message)
}

#[cfg(test)]
pub(super) fn service_request_with_context(
    context: SidecarRequestContext,
    capability_version: impl Into<String>,
    payload: impl Serialize,
) -> Result<JsonRequest, String> {
    let payload = serde_json::to_value(payload).map_err(|err| err.to_string())?;
    service_request_with_value_context(context, capability_version, payload)
}

pub(in crate::sidecar_clients) fn service_request_with_value_context(
    context: SidecarRequestContext,
    capability_version: impl Into<String>,
    payload: Value,
) -> Result<JsonRequest, String> {
    service_request_with_typed_context(context, capability_version, payload)
}

pub(crate) fn service_request_with_typed_context<T: Serialize>(
    context: SidecarRequestContext,
    capability_version: impl Into<String>,
    payload: T,
) -> Result<ServiceRequest<T>, String> {
    let payload_value = serde_json::to_value(&payload).map_err(|err| err.to_string())?;
    let capability_version = capability_version.into();
    resolved_service_request_context(context, &capability_version, &payload_value)?
        .with_payload(capability_version, payload)
        .map_err(|error| error.message)
}

fn resolved_service_request_context(
    mut context: SidecarRequestContext,
    capability_version: &str,
    payload: &Value,
) -> Result<ServiceRequestContext, String> {
    let (project_id, root_project_id) = payload_project_scope(payload)?;
    merge_project_scope("project_id", &mut context.project_id, project_id)?;
    merge_project_scope(
        "root_project_id",
        &mut context.root_project_id,
        root_project_id,
    )?;
    let secret_policy = resolve_egress_policy(capability_version, context.secret_policy)?;
    let request_id = uuid::Uuid::new_v4().to_string();
    let trace_context = context.trace_context.unwrap_or_else(|| TraceContext {
        trace_id: request_id.clone(),
        parent_span_id: None,
    });
    let mut request = ServiceRequestContext::new(request_id, trace_context, secret_policy);
    request.user_id = context.user_id;
    request.project_id = context.project_id;
    request.root_project_id = context.root_project_id;
    request.runtime_config_snapshot = context.runtime_config_snapshot;
    Ok(request)
}

fn resolve_egress_policy(
    capability_version: &str,
    policy: SecretForwardingPolicy,
) -> Result<SecretForwardingPolicy, String> {
    let (service, url) = sidecar_service_and_url_for_capability(capability_version)?;
    let policy = if policy.is_deny_all() {
        if is_loopback_http_url(&url) {
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
            service = %service,
            data_type = ?event.data_type,
            audit_event = event.audit_event,
            "authorized trusted-remote sidecar egress"
        );
    }
    Ok(policy)
}

pub(super) fn ensure_global_request_scope<T>(request: &ServiceRequest<T>) -> Result<(), String> {
    if request.project_id.is_some() || request.root_project_id.is_some() {
        return Err(format!(
            "global sidecar request 不能携带项目作用域: project_id={:?}, root_project_id={:?}",
            request.project_id, request.root_project_id
        ));
    }
    Ok(())
}
