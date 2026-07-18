//! Shared protocol types for A4N sidecar services.
//!
//! This crate intentionally contains transport-neutral contracts only. Business
//! API DTOs stay in host/orchestrator-facing crates until they are stable.

mod artifacts;
mod events;
mod health;
mod registry;
mod request;
mod response;
mod sanitization;
mod validation;

pub use artifacts::{
    ArtifactLocation, ArtifactOwnership, ArtifactRef, ArtifactValidationError,
    validate_artifact_ref, validate_artifact_refs,
};
pub use events::{ServiceTaskEvent, ServiceTaskEventKind};
pub use health::{
    HealthStatus, ServiceHealth, ServiceHealthValidationError, ServiceVersion,
    validate_service_health_envelope,
};
pub use registry::{
    EMBEDDING_CAPABILITY_VERSION, EMBEDDING_SERVICE_NAME, EXPORT_RENDER_CAPABILITY_VERSION,
    EXPORT_SERVICE_NAME, IMAGE_PROVIDER_CAPABILITY_VERSION, IMAGE_SERVICE_NAME,
    IMPORT_SERVICE_NAME, IMPORT_TRANSFORM_CAPABILITY_VERSION, LLM_CHAT_CAPABILITY_VERSION,
    LLM_SERVICE_NAME, PROMPT_RENDER_CAPABILITY_VERSION, PROMPT_SERVICE_NAME, PROTOCOL_VERSION,
    RAG_INDEX_CAPABILITY_VERSION, RAG_SERVICE_NAME, VIDEO_PROVIDER_CAPABILITY_VERSION,
    VIDEO_SERVICE_NAME, expected_capability_label_for_service,
    expected_capability_version_for_service, known_service_health,
    required_secret_policy_for_service,
};
pub use request::{
    SchemaVersion, SecretForwardingPolicy, SecretMaterialScope, ServiceRequest,
    ServiceRequestContext, TraceContext,
};
pub use response::{ServiceError, ServiceErrorCode, ServiceResponse, ServiceStatus};
pub use sanitization::{
    contains_unredacted_sensitive_value, redact_sensitive_message, redact_sensitive_value,
    sanitize_service_response, sanitize_service_task_event,
};
pub use validation::{
    RequiredSecretPolicy, validate_known_service_request_envelope,
    validate_provider_api_key_secret_policy, validate_service_request_context,
    validate_service_request_envelope,
};

#[cfg(test)]
mod tests;
