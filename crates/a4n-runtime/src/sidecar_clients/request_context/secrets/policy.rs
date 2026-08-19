use super::detection::payload_has_provider_secret;
use crate::sidecar_clients::{
    request_context::{
        context::SidecarRequestContext,
        requests::{ensure_global_request_scope, service_request_with_value_context},
    },
    service_manager::sidecar_service_and_url_for_capability,
};
use a4n_capabilities::JsonRequest;
use a4n_service_manager::is_loopback_http_url;
use a4n_service_protocol::SecretForwardingPolicy;
use serde::Serialize;

pub(crate) fn global_provider_service_request(
    capability_version: impl Into<String>,
    payload: impl Serialize,
) -> Result<JsonRequest, String> {
    let request = service_request_with_context_provider_secret_policy(
        SidecarRequestContext::current_global(),
        capability_version,
        payload,
    )?;
    ensure_global_request_scope(&request)?;
    Ok(request)
}

pub(crate) fn service_request_with_context_provider_secret_policy(
    context: SidecarRequestContext,
    capability_version: impl Into<String>,
    payload: impl Serialize,
) -> Result<JsonRequest, String> {
    let capability_version = capability_version.into();
    let (_, sidecar_url) = sidecar_service_and_url_for_capability(&capability_version)?;
    service_request_with_context_provider_secret_policy_for_url(
        context,
        &sidecar_url,
        capability_version,
        payload,
    )
}

#[cfg(test)]
pub(in crate::sidecar_clients::request_context) fn global_provider_service_request_for_url(
    sidecar_url: &str,
    capability_version: impl Into<String>,
    payload: impl Serialize,
) -> Result<JsonRequest, String> {
    let request = service_request_with_context_provider_secret_policy_for_url(
        SidecarRequestContext::current_global(),
        sidecar_url,
        capability_version,
        payload,
    )?;
    ensure_global_request_scope(&request)?;
    Ok(request)
}

pub(in crate::sidecar_clients::request_context) fn service_request_with_context_provider_secret_policy_for_url(
    context: SidecarRequestContext,
    sidecar_url: &str,
    capability_version: impl Into<String>,
    payload: impl Serialize,
) -> Result<JsonRequest, String> {
    let payload = serde_json::to_value(payload).map_err(|err| err.to_string())?;
    if !payload_has_provider_secret(&payload) {
        let policy = if is_loopback_http_url(sidecar_url) {
            SecretForwardingPolicy::deny_all()
        } else {
            SecretForwardingPolicy::trusted_remote_no_secrets()
        };
        return service_request_with_value_context(
            context.with_secret_policy(policy),
            capability_version,
            payload,
        );
    }
    if !is_loopback_http_url(sidecar_url) {
        return Err(
            "provider sidecar URL 不是本地 loopback 地址，拒绝转发 provider secret".to_string(),
        );
    }
    service_request_with_value_context(
        context.with_secret_policy(SecretForwardingPolicy::local_user_secret()),
        capability_version,
        payload,
    )
}
