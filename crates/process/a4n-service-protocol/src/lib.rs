//! Shared protocol types for A4N sidecar services.
//!
//! This crate intentionally contains transport-neutral contracts only. Business
//! API DTOs stay in host/orchestrator-facing crates until they are stable.

mod artifacts;
mod egress;
mod events;
mod health;
mod registry;
mod request;
mod resource;
mod response;
mod runtime;
mod sanitization;
mod validation;

pub use artifacts::{
    ArtifactLocation, ArtifactOwnership, ArtifactRef, ArtifactValidationError,
    validate_artifact_refs,
};
pub const SIDECAR_AUTH_TOKEN_ENV: &str = "A4N_SIDECAR_AUTH_TOKEN";
pub const SIDECAR_AUTH_HEADER: &str = "x-a4n-sidecar-token";
pub const SIDECAR_HEALTH_PATH: &str = "/healthz";
pub const SIDECAR_METRICS_PATH: &str = "/metrics";
pub const SIDECAR_SHUTDOWN_PATH: &str = "/shutdown";
pub const SIDECAR_EVENT_CONTENT_TYPE: &str = "application/x-ndjson";
pub const EMBEDDING_EMBED_TEXTS_PATH: &str = "/embedding/embed_texts";
pub const EMBEDDING_MODELS_PATH: &str = "/embedding/models";
pub const EMBEDDING_MODEL_DOWNLOAD_INFO_PATH: &str = "/embedding/model/download_info";
pub const EMBEDDING_MODEL_DOWNLOAD_EVENTS_PATH: &str = "/embedding/model/download/events";
pub const EMBEDDING_MODEL_DELETE_PATH: &str = "/embedding/model/delete";
pub const RAG_QUERY_PATH: &str = "/rag/query";
pub const RAG_SEARCH_TEXT_PATH: &str = "/rag/search_text";
pub const RAG_INGEST_EVENTS_PATH: &str = "/rag/ingest/events";
pub const RAG_SEARCH_PATH: &str = "/rag/search";
pub const RAG_DELETE_PATH: &str = "/rag/delete";
pub const RAG_DROP_COLLECTION_PATH: &str = "/rag/drop_collection";
pub const RAG_METADATA_BY_IDS_PATH: &str = "/rag/metadata_by_ids";
pub const RAG_METADATA_BY_FILTER_PATH: &str = "/rag/metadata_by_filter";
pub const RAG_COMPLETENESS_REPORT_BY_IDS_PATH: &str = "/rag/completeness_report_by_ids";
pub const SIDECAR_REQUEST_ID_HEADER: &str = "x-a4n-request-id";
pub const SIDECAR_TRACE_ID_HEADER: &str = "x-a4n-trace-id";
pub const SIDECAR_RESTART_COUNT_ENV: &str = "A4N_SIDECAR_RESTART_COUNT";
pub const SIDECAR_LAST_ERROR_ENV: &str = "A4N_SIDECAR_LAST_ERROR";
pub const MANAGED_SIDECARS_ENV: &str = "A4N_MANAGED_SIDECARS";
pub const SIDECAR_BIN_DIR_ENV: &str = "A4N_SIDECAR_BIN_DIR";
pub const EMBEDDING_CACHE_DIR_ENV: &str = "A4N_EMBEDDING_CACHE_DIR";
pub const PROMPT_SERVICE_PROMPTS_DIR_ENV: &str = "A4N_PROMPT_SERVICE_PROMPTS_DIR";
pub const RELEASE_MANIFEST_ENV: &str = "A4N_RELEASE_MANIFEST_PATH";

pub fn parse_service_socket_addr(key: &str, value: &str) -> Result<std::net::SocketAddr, String> {
    let value = value.trim();
    if value.is_empty() {
        return Err(format!("{key} 不能为空"));
    }
    value
        .parse()
        .map_err(|error| format!("{key} 必须是有效监听地址: {error}"))
}

pub use a4n_generation_artifact_contract::GenerationArtifactRef;
pub use egress::{EgressAuditEvent, EgressDataType, validate_egress_scope};
pub use events::{
    capability::{CapabilityTaskEvent, CapabilityTaskEventKind},
    generation::{
        ArtifactEvent, ArtifactOperation, BatchCommitPolicy, BatchSummary, CancelRequestedEvent,
        CancellationReason, CancelledEvent, CheckpointEvent, CompletedEvent,
        DEFAULT_GENERATION_ACTIVE_SESSION_MAX, DraftDisposition, FailedEvent,
        GENERATION_EVENT_SCHEMA_VERSION, GenerationContractError, GenerationDeliveryMode,
        GenerationError, GenerationErrorCode, GenerationEvent, GenerationFinishReason,
        GenerationResultRef, GenerationTargetRef, GenerationTaskEvent, GenerationTaskSnapshot,
        GenerationTaskStatus, GenerationUsage, InterruptedEvent, InterruptionReason,
        JS_SAFE_INTEGER_I64, JS_SAFE_INTEGER_U64, MAX_GENERATION_EVENT_BYTES,
        MAX_GENERATION_SNAPSHOT_BYTES, MAX_TEXT_DELTA_BYTES, ProgressEvent, StartedEvent,
        StatusChangedEvent, StreamTextHash, TextDeltaEvent, TextStreamSnapshot, UsageEvent,
    },
};
pub use health::{
    HealthStatus, ServiceHealth, ServiceHealthValidationError, ServiceVersion,
    validate_service_health_envelope,
};
pub use registry::{
    EMBEDDING_CAPABILITY_VERSION, EMBEDDING_SERVICE_NAME, EXPORT_RENDER_CAPABILITY_VERSION,
    EXPORT_SERVICE_NAME, IMAGE_PROVIDER_CAPABILITY_VERSION, IMAGE_SERVICE_NAME,
    IMPORT_SERVICE_NAME, IMPORT_TRANSFORM_CAPABILITY_VERSION, LLM_CHAT_CAPABILITY_VERSION,
    LLM_SERVICE_NAME, MEDIA_FFMPEG_CAPABILITY_VERSION, MEDIA_SERVICE_NAME,
    PROMPT_RENDER_CAPABILITY_VERSION, PROMPT_SERVICE_NAME, PROTOCOL_VERSION,
    RAG_INDEX_CAPABILITY_VERSION, RAG_SERVICE_NAME, VIDEO_PROVIDER_CAPABILITY_VERSION,
    VIDEO_SERVICE_NAME, egress_data_type_for_service, expected_capability_version_for_service,
    known_service_health, service_for_capability_version,
};
pub use request::{
    SchemaVersion, SecretForwardingPolicy, SecretMaterialScope, ServiceRequest,
    ServiceRequestContext, TraceContext,
};
pub use resource::{
    BodyLimitKind, SIDECAR_CONNECT_TIMEOUT, SIDECAR_EVENT_QUEUE_CAPACITY, SIDECAR_HEALTH_TIMEOUT,
    SIDECAR_REQUEST_TIMEOUT, SIDECAR_STREAM_IDLE_TIMEOUT, SidecarResourcePolicy,
    sidecar_resource_policy,
};
pub use response::{ServiceError, ServiceErrorCode, ServiceResponse, ServiceStatus};
pub use runtime::{ServiceOperationMetrics, ServiceRuntimeMetrics};
pub use sanitization::{
    redact_sensitive_message, redact_sensitive_value, sanitize_capability_task_event,
    sanitize_service_response,
};
pub use validation::validate_service_request_context;

#[cfg(test)]
mod tests;
