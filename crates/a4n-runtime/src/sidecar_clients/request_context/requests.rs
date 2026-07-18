use a4n_capabilities::JsonRequest;
use a4n_service_protocol::{
    SecretForwardingPolicy, ServiceRequest, TraceContext, validate_service_request_context,
};
use serde::Serialize;
use serde_json::Value;

use super::{
    context::SidecarRequestContext,
    scope::{merge_project_scope, payload_project_scope},
    trace::request_id,
};

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
        secret_policy: context.secret_policy,
        capability_version: capability_version.into(),
        payload,
    };
    validate_service_request_context(&request).map_err(|error| error.message)?;
    Ok(request)
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
