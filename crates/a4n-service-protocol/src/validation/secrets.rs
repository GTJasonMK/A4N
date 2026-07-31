use crate::registry::{IMAGE_SERVICE_NAME, LLM_SERVICE_NAME, VIDEO_SERVICE_NAME};
use crate::request::{SecretForwardingPolicy, SecretMaterialScope};
use crate::response::{ServiceError, ServiceErrorCode};
use crate::sanitization::redact_sensitive_message;
use crate::{EgressDataType, validate_egress_scope};

use super::errors::unauthorized_error;

pub fn validate_provider_api_key_secret_policy(
    api_key: Option<&str>,
    policy: &SecretForwardingPolicy,
    service: &str,
) -> Result<(), ServiceError> {
    let has_user_secret = api_key.is_some_and(|key| !key.trim().is_empty());
    if !has_user_secret {
        return Ok(());
    }

    if policy.is_local_user_secret_only() {
        return Ok(());
    }

    let message = provider_api_key_secret_policy_rejection_message(service).ok_or_else(|| {
        ServiceError::non_retryable(
            ServiceErrorCode::ConfigurationError,
            redact_sensitive_message(format!(
                "unknown provider sidecar service for api_key secret policy: {service}"
            )),
        )
    })?;

    if matches!(policy.scope, SecretMaterialScope::TrustedRemote) {
        validate_egress_scope(service, EgressDataType::ProviderSecret, &policy.scope)
            .map_err(|_| unauthorized_error(message.to_string()))?;
    }

    Err(unauthorized_error(message.to_string()))
}

fn provider_api_key_secret_policy_rejection_message(service: &str) -> Option<&'static str> {
    match service {
        LLM_SERVICE_NAME => Some(
            "LLM provider API key requires user-secret-only policy: scope=local_sidecar_only, allow_user_secret=true, allow_system_secret=false, allow_upload_token=false",
        ),
        IMAGE_SERVICE_NAME => Some(
            "image provider api_key requires user-secret-only policy: scope=local_sidecar_only, allow_user_secret=true, allow_system_secret=false, allow_upload_token=false",
        ),
        VIDEO_SERVICE_NAME => Some(
            "video provider api_key requires user-secret-only policy: scope=local_sidecar_only, allow_user_secret=true, allow_system_secret=false, allow_upload_token=false",
        ),
        _ => None,
    }
}
