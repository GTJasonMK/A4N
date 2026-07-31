use crate::request::SecretMaterialScope;
use crate::response::{ServiceError, ServiceErrorCode};
use crate::sanitization::redact_sensitive_message;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EgressDataType {
    Metadata,
    Prompt,
    Text,
    Image,
    Video,
    Archive,
    ProviderSecret,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EgressPolicy {
    pub allow_trusted_remote: bool,
    pub audit_event: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EgressAuditEvent {
    pub service: String,
    pub data_type: EgressDataType,
    pub allowed: bool,
    pub audit_event: &'static str,
    pub reason: &'static str,
}

pub fn egress_policy_for(service: &str, data_type: EgressDataType) -> EgressPolicy {
    let allow_trusted_remote = matches!(data_type, EgressDataType::Metadata)
        || matches!(
            (service, data_type),
            ("prompt", EgressDataType::Prompt)
                | ("llm" | "embedding" | "rag", EgressDataType::Text)
                | ("image", EgressDataType::Image)
                | ("video", EgressDataType::Video)
                | ("import" | "export", EgressDataType::Archive)
        );
    EgressPolicy {
        allow_trusted_remote,
        audit_event: match (data_type, allow_trusted_remote) {
            (EgressDataType::Metadata, _) => "sidecar.egress.metadata_allowed",
            (_, true) => "sidecar.egress.content_allowed",
            (_, false) => "sidecar.egress.content_denied",
        },
    }
}

pub fn validate_egress_scope(
    service: &str,
    data_type: EgressDataType,
    scope: &SecretMaterialScope,
) -> Result<EgressAuditEvent, ServiceError> {
    let policy = egress_policy_for(service, data_type);
    let allowed =
        !matches!(scope, SecretMaterialScope::TrustedRemote) || policy.allow_trusted_remote;
    let reason = if allowed {
        "explicit egress policy allowed"
    } else {
        "content egress to trusted remote is denied by default"
    };
    let event = EgressAuditEvent {
        service: service.to_string(),
        data_type,
        allowed,
        audit_event: policy.audit_event,
        reason,
    };
    if allowed {
        return Ok(event);
    }
    Err(ServiceError::non_retryable(
        ServiceErrorCode::Unauthorized,
        redact_sensitive_message(format!(
            "{}: service={} data_type={data_type:?}",
            reason, service
        )),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remote_content_is_denied_without_an_explicit_capability_exception() {
        let error = validate_egress_scope(
            "llm",
            EgressDataType::Prompt,
            &SecretMaterialScope::TrustedRemote,
        )
        .expect_err("prompt content must not leave the host by default");
        assert_eq!(error.code, ServiceErrorCode::Unauthorized);
        assert!(!error.message.contains("prompt"));
    }

    #[test]
    fn metadata_can_be_sent_to_a_trusted_remote_and_is_auditable() {
        let event = validate_egress_scope(
            "llm",
            EgressDataType::Metadata,
            &SecretMaterialScope::TrustedRemote,
        )
        .expect("metadata is the only default remote egress");
        assert!(event.allowed);
        assert_eq!(event.audit_event, "sidecar.egress.metadata_allowed");
    }

    #[test]
    fn capability_and_data_type_pair_is_the_explicit_remote_authorization() {
        let event = validate_egress_scope(
            "llm",
            EgressDataType::Text,
            &SecretMaterialScope::TrustedRemote,
        )
        .expect("LLM text egress is explicitly registered");
        assert!(event.allowed);

        validate_egress_scope(
            "llm",
            EgressDataType::ProviderSecret,
            &SecretMaterialScope::TrustedRemote,
        )
        .expect_err("provider secrets never follow remote content");
    }
}
