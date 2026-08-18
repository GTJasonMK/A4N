mod detection;
mod policy;

pub(crate) use policy::{
    global_provider_service_request, service_request_with_context_provider_secret_policy,
};

#[cfg(test)]
pub(super) use detection::payload_has_provider_secret;
#[cfg(test)]
pub(super) use policy::{
    global_provider_service_request_for_url,
    service_request_with_context_provider_secret_policy_for_url,
};
