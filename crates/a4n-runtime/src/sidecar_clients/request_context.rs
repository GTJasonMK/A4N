mod context;
mod requests;
mod scope;
mod secrets;
mod trace;

pub(crate) use context::SidecarRequestContext;
#[cfg(test)]
pub(crate) use requests::pure_transform_service_request;
pub(crate) use requests::{
    global_service_request_without_secrets, pure_transform_typed_service_request,
    service_request_with_context,
};
pub(super) use scope::value_as_i64;
pub(crate) use secrets::{
    global_provider_service_request, service_request_with_context_provider_secret_policy,
};
pub(crate) use trace::sidecar_context_for_llm_trace;
pub(super) use trace::sidecar_context_for_workflow_operation_with_root;

#[cfg(test)]
use crate::runtime_config;
#[cfg(test)]
use a4n_llm::provider::LlmTraceContext;
#[cfg(test)]
use a4n_service_protocol::{SecretForwardingPolicy, TraceContext};
#[cfg(test)]
use secrets::{
    global_provider_service_request_for_url, payload_has_provider_secret,
    service_request_with_context_provider_secret_policy_for_url, sidecar_url_allows_local_secrets,
};

#[cfg(test)]
mod tests;
