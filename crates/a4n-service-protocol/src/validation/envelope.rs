use crate::registry::{
    PROTOCOL_VERSION, expected_capability_label_for_service,
    expected_capability_version_for_service, required_secret_policy_for_service,
};
use crate::request::ServiceRequest;
use crate::response::ServiceError;

use super::context::validate_service_request_context;
use super::errors::{unauthorized_error, version_mismatch_error};
use super::policy::RequiredSecretPolicy;

pub fn validate_service_request_envelope<T>(
    request: &ServiceRequest<T>,
    expected_capability_version: &str,
    capability_label: &str,
    required_secret_policy: RequiredSecretPolicy,
) -> Result<(), ServiceError> {
    if request.schema_version.protocol != PROTOCOL_VERSION {
        return Err(version_mismatch_error(format!(
            "unsupported protocol version: expected {}, got {}",
            PROTOCOL_VERSION, request.schema_version.protocol
        )));
    }

    if !request.schema_version.is_protocol_only() {
        return Err(version_mismatch_error(format!(
            "sidecar request envelope is protocol-only and does not accept database schema version: got {}",
            request
                .schema_version
                .database
                .as_deref()
                .unwrap_or_default()
        )));
    }

    validate_service_request_context(request)?;

    if request.capability_version != expected_capability_version {
        return Err(version_mismatch_error(format!(
            "unsupported {capability_label} version: expected {expected_capability_version}, got {}",
            request.capability_version
        )));
    }

    match required_secret_policy {
        RequiredSecretPolicy::AllowAny => {}
        RequiredSecretPolicy::DenyAll { rejection_message } => {
            if !request.secret_policy.is_secret_free() {
                return Err(unauthorized_error(rejection_message.to_string()));
            }
        }
        RequiredSecretPolicy::LocalUserSecretForwarding { rejection_message } => {
            if !request.secret_policy.is_local_user_secret_only() {
                return Err(unauthorized_error(rejection_message.to_string()));
            }
        }
    }

    Ok(())
}

pub fn validate_known_service_request_envelope<T>(
    request: &ServiceRequest<T>,
    service: &str,
) -> Result<(), ServiceError> {
    let expected_capability_version =
        expected_capability_version_for_service(service).ok_or_else(|| {
            version_mismatch_error(format!(
                "unknown sidecar service for request envelope: {service}"
            ))
        })?;
    let capability_label = expected_capability_label_for_service(service).ok_or_else(|| {
        version_mismatch_error(format!(
            "unknown sidecar service for request envelope: {service}"
        ))
    })?;
    let required_secret_policy = required_secret_policy_for_service(service).ok_or_else(|| {
        version_mismatch_error(format!(
            "unknown sidecar service for request envelope: {service}"
        ))
    })?;

    validate_service_request_envelope(
        request,
        expected_capability_version,
        capability_label,
        required_secret_policy,
    )
}
