use crate::{
    HealthStatus, ServiceHealth, ServiceRequest, ServiceRequestContext, ServiceVersion,
    TraceContext,
};

pub(super) fn service_request<T>(capability_version: &str, payload: T) -> ServiceRequest<T> {
    ServiceRequestContext::deny_all(
        "request-1",
        TraceContext {
            trace_id: "trace-1".to_string(),
            parent_span_id: None,
        },
    )
    .with_payload(capability_version, payload)
    .expect("test service request context should be valid")
}

pub(super) fn service_health(
    service: &str,
    status: HealthStatus,
    protocol_version: u32,
    capability_version: &str,
) -> ServiceHealth {
    ServiceHealth {
        service: service.to_string(),
        status,
        version: ServiceVersion {
            service_version: "test".to_string(),
            protocol_version,
            capability_version: capability_version.to_string(),
        },
    }
}
