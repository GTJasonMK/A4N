use super::contracts::HealthStatus;
use crate::response::{ServiceError, ServiceErrorCode};
use crate::sanitization::redact_sensitive_message;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServiceHealthValidationError {
    ServiceIdentityMismatch {
        expected: String,
        actual: String,
    },
    Unhealthy {
        service: String,
        status: HealthStatus,
    },
    ProtocolMismatch {
        service: String,
        expected: u32,
        actual: u32,
    },
    CapabilityMismatch {
        service: String,
        expected: String,
        actual: String,
    },
}

impl ServiceHealthValidationError {
    pub fn service_error_code(&self) -> ServiceErrorCode {
        match self {
            Self::ServiceIdentityMismatch { .. }
            | Self::ProtocolMismatch { .. }
            | Self::CapabilityMismatch { .. } => ServiceErrorCode::VersionMismatch,
            Self::Unhealthy { .. } => ServiceErrorCode::ServiceUnavailable,
        }
    }

    pub fn retryable(&self) -> bool {
        matches!(self, Self::Unhealthy { .. })
    }

    pub fn message(&self) -> String {
        match self {
            Self::ServiceIdentityMismatch { expected, actual } => {
                format!("sidecar service identity mismatch: expected {expected}, got {actual}")
            }
            Self::Unhealthy { service, status } => {
                format!("sidecar {service} is unhealthy: {status:?}")
            }
            Self::ProtocolMismatch {
                service,
                expected,
                actual,
            } => {
                format!(
                    "sidecar {service} protocol version mismatch: expected {expected}, got {actual}"
                )
            }
            Self::CapabilityMismatch {
                service,
                expected,
                actual,
            } => {
                format!(
                    "sidecar {service} capability version mismatch: expected {expected}, got {actual}"
                )
            }
        }
    }

    pub fn into_service_error(self) -> ServiceError {
        ServiceError::new(
            self.service_error_code(),
            redact_sensitive_message(self.message()),
            self.retryable(),
        )
    }
}
