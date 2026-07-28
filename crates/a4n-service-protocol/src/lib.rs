//! Shared protocol types for A4N sidecar services.
//!
//! This crate intentionally contains transport-neutral contracts only. Business
//! API DTOs stay in host/orchestrator-facing crates until they are stable.

mod artifacts;
mod catalog;
mod compatibility;
mod egress;
pub mod events;
mod health;
mod registry;
mod release;
mod request;
mod resource;
mod response;
mod runtime;
mod sanitization;
mod validation;

pub use artifacts::{
    ArtifactLocation, ArtifactOwnership, ArtifactRef, ArtifactValidationError,
    validate_artifact_ref, validate_artifact_refs,
};
pub const SIDECAR_AUTH_TOKEN_ENV: &str = "A4N_SIDECAR_AUTH_TOKEN";
pub const SIDECAR_AUTH_HEADER: &str = "x-a4n-sidecar-token";
pub const SIDECAR_HOST_PROTOCOL_VERSION_HEADER: &str = "x-a4n-host-protocol-version";
pub const SIDECAR_REQUEST_ID_HEADER: &str = "x-a4n-request-id";
pub const SIDECAR_TRACE_ID_HEADER: &str = "x-a4n-trace-id";
pub const SIDECAR_RESTART_COUNT_ENV: &str = "A4N_SIDECAR_RESTART_COUNT";
pub const SIDECAR_LAST_ERROR_ENV: &str = "A4N_SIDECAR_LAST_ERROR";
pub const RELEASE_MANIFEST_ENV: &str = "A4N_RELEASE_MANIFEST_PATH";
pub use catalog::{ServiceCatalogEntry, service_catalog, service_catalog_entry};
pub use compatibility::{
    DeploymentMode, NegotiatedServiceVersion, ProtocolChangeKind, SUPPORTED_PROTOCOL_RANGE,
    ServiceCompatibility, VersionNegotiationError, VersionRange, capability_major,
    negotiate_deployment_version, negotiate_service_version, service_compatibility,
    service_compatibility_for_host,
};
pub use egress::{
    EgressAuditEvent, EgressDataType, EgressPolicy, egress_policy_for, validate_egress_scope,
};
pub use events::*;
pub use health::{
    HealthStatus, ServiceHealth, ServiceHealthValidationError, ServiceVersion,
    negotiate_service_health_envelope, validate_service_health_envelope,
};
pub use registry::{
    EMBEDDING_CAPABILITY_VERSION, EMBEDDING_SERVICE_NAME, EXPORT_RENDER_CAPABILITY_VERSION,
    EXPORT_SERVICE_NAME, IMAGE_PROVIDER_CAPABILITY_VERSION, IMAGE_SERVICE_NAME,
    IMPORT_SERVICE_NAME, IMPORT_TRANSFORM_CAPABILITY_VERSION, LLM_CHAT_CAPABILITY_VERSION,
    LLM_SERVICE_NAME, PROMPT_RENDER_CAPABILITY_VERSION, PROMPT_SERVICE_NAME, PROTOCOL_VERSION,
    RAG_INDEX_CAPABILITY_VERSION, RAG_SERVICE_NAME, VIDEO_PROVIDER_CAPABILITY_VERSION,
    VIDEO_SERVICE_NAME, egress_data_type_for_service, expected_capability_label_for_service,
    expected_capability_version_for_service, known_service_health, known_service_health_for_host,
    required_secret_policy_for_service, service_for_capability_version,
};
pub use release::{ReleaseBinaryManifest, ReleaseManifest, ReleaseRuntimeFileManifest};
pub use request::{
    SchemaVersion, SecretForwardingPolicy, SecretMaterialScope, ServiceRequest,
    ServiceRequestContext, TraceContext,
};
pub use resource::{
    BodyLimitKind, SIDECAR_CONNECT_TIMEOUT, SIDECAR_EVENT_QUEUE_CAPACITY, SIDECAR_HEALTH_TIMEOUT,
    SIDECAR_REQUEST_TIMEOUT, SIDECAR_STREAM_IDLE_TIMEOUT, SidecarResourcePolicy,
    resource_policy_for_service, sidecar_resource_policy,
};
pub use response::{ServiceError, ServiceErrorCode, ServiceResponse, ServiceStatus};
pub use runtime::{ServiceOperationMetrics, ServiceRuntimeMetrics};
pub use sanitization::{
    contains_unredacted_sensitive_value, redact_sensitive_message, redact_sensitive_value,
    sanitize_capability_task_event, sanitize_service_response,
};
pub use validation::{
    RequiredSecretPolicy, validate_known_service_request_envelope,
    validate_provider_api_key_secret_policy, validate_service_request_context,
    validate_service_request_envelope,
};

#[cfg(test)]
mod tests;
