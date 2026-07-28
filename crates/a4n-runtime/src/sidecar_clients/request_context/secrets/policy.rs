use super::{detection::payload_has_provider_secret, url::sidecar_url_allows_local_secrets};
use crate::sidecar_clients::{
    request_context::{
        context::SidecarRequestContext,
        requests::{ensure_global_request_scope, service_request_value_with_context_and_policy},
    },
    service_manager::sidecar_url_for_service,
};
use a4n_capabilities::JsonRequest;
use a4n_service_protocol::SecretForwardingPolicy;
use serde::Serialize;

pub(crate) fn global_provider_service_request(
    service_name: &str,
    capability_version: impl Into<String>,
    payload: impl Serialize,
) -> Result<JsonRequest, String> {
    let sidecar_url = sidecar_url_for_service(service_name)?;
    global_provider_service_request_for_url(service_name, &sidecar_url, capability_version, payload)
}

pub(crate) fn service_request_with_context_provider_secret_policy(
    context: SidecarRequestContext,
    service_name: &str,
    capability_version: impl Into<String>,
    payload: impl Serialize,
) -> Result<JsonRequest, String> {
    let sidecar_url = sidecar_url_for_service(service_name)?;
    service_request_with_context_provider_secret_policy_for_url(
        context,
        service_name,
        &sidecar_url,
        capability_version,
        payload,
    )
}

pub(in crate::sidecar_clients::request_context) fn global_provider_service_request_for_url(
    service_name: &str,
    sidecar_url: &str,
    capability_version: impl Into<String>,
    payload: impl Serialize,
) -> Result<JsonRequest, String> {
    let request = service_request_with_context_provider_secret_policy_for_url(
        SidecarRequestContext::current_global(),
        service_name,
        sidecar_url,
        capability_version,
        payload,
    )?;
    ensure_global_request_scope(&request)?;
    Ok(request)
}

pub(in crate::sidecar_clients::request_context) fn service_request_with_context_provider_secret_policy_for_url(
    context: SidecarRequestContext,
    service_name: &str,
    sidecar_url: &str,
    capability_version: impl Into<String>,
    payload: impl Serialize,
) -> Result<JsonRequest, String> {
    let payload = serde_json::to_value(payload).map_err(|err| err.to_string())?;
    if !payload_has_provider_secret(&payload) {
        let policy = if sidecar_url_allows_local_secrets(sidecar_url) {
            SecretForwardingPolicy::deny_all()
        } else {
            SecretForwardingPolicy::trusted_remote_no_secrets()
        };
        return service_request_value_with_context_and_policy(
            context,
            capability_version,
            payload,
            policy,
        );
    }
    if !sidecar_url_allows_local_secrets(sidecar_url) {
        return Err(format!(
            "{service_name} sidecar URL 不是本地 loopback 地址，拒绝转发 provider secret"
        ));
    }
    service_request_value_with_context_and_policy(
        context,
        capability_version,
        payload,
        SecretForwardingPolicy::local_user_secret(),
    )
}
