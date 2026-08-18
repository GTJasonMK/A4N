use a4n_service_protocol::{
    CapabilityTaskEvent, CapabilityTaskEventKind, SIDECAR_EVENT_CONTENT_TYPE, ServiceError,
    ServiceErrorCode,
};
use axum::http::header;

use super::{failed_service_event_response, service_event_line};

#[test]
fn service_event_line_is_newline_delimited_json() {
    let line = service_event_line(CapabilityTaskEvent::completed_empty(
        "task-1".to_string(),
        1,
    ));
    assert_eq!(line.last(), Some(&b'\n'));
    let event: CapabilityTaskEvent =
        serde_json::from_slice(&line[..line.len() - 1]).expect("event JSON");
    assert_eq!(event.task_id, "task-1");
    assert_eq!(event.event_kind, CapabilityTaskEventKind::Completed);
}

#[test]
fn service_event_response_uses_ndjson_content_type() {
    let response = failed_service_event_response(
        "task-1".to_string(),
        ServiceError::non_retryable(ServiceErrorCode::Internal, "failed"),
    );
    assert_eq!(
        response.headers().get(header::CONTENT_TYPE),
        Some(&header::HeaderValue::from_static(
            SIDECAR_EVENT_CONTENT_TYPE
        ))
    );
}
