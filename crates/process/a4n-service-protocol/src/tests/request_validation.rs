mod context;
mod envelope;
mod registry;
mod secret_policy;

use serde_json::json;

use super::support::service_request;
use crate::validation::{
    RequiredSecretPolicy, validate_known_service_request_envelope,
    validate_provider_api_key_secret_policy, validate_service_request_envelope,
};
use crate::{
    HealthStatus, IMAGE_PROVIDER_CAPABILITY_VERSION, IMAGE_SERVICE_NAME, IMPORT_SERVICE_NAME,
    IMPORT_TRANSFORM_CAPABILITY_VERSION, LLM_CHAT_CAPABILITY_VERSION, LLM_SERVICE_NAME,
    PROMPT_RENDER_CAPABILITY_VERSION, PROMPT_SERVICE_NAME, PROTOCOL_VERSION,
    SecretForwardingPolicy, SecretMaterialScope, ServiceErrorCode, ServiceRequestContext,
    ServiceStatus, TraceContext, VIDEO_SERVICE_NAME, known_service_health,
    validate_service_request_context,
};

#[test]
fn execute_known_service_rejects_before_running_the_operation() {
    let mut request = service_request("import-transform-v0", json!({}));
    request.trace_context.trace_id = "trace-execute".to_string();

    let response = request.execute_known_service(
        IMPORT_SERVICE_NAME,
        |_| -> Result<(), &str> { panic!("invalid request must not execute") },
        |_| unreachable!(),
    );

    assert_eq!(response.status, ServiceStatus::Error);
    assert_eq!(response.trace_id.as_deref(), Some("trace-execute"));
    assert_eq!(
        response.error.expect("version error").code,
        ServiceErrorCode::VersionMismatch
    );
}
