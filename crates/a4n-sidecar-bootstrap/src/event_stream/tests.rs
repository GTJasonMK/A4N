use a4n_service_protocol::{CapabilityTaskEvent, CapabilityTaskEventKind};
use axum::http::header;

use super::{service_event_line, service_event_ndjson_response, single_service_event_stream};

#[test]
fn service_event_line_is_newline_delimited_json() {
    let line = service_event_line(test_event());
    assert_eq!(line.last(), Some(&b'\n'));
    let event: CapabilityTaskEvent =
        serde_json::from_slice(&line[..line.len() - 1]).expect("event JSON");
    assert_eq!(event.task_id, "task-1");
    assert_eq!(event.event_kind, CapabilityTaskEventKind::Completed);
}

#[test]
fn service_event_response_uses_ndjson_content_type() {
    let response =
        service_event_ndjson_response(single_service_event_stream(test_event()), "test service");
    assert_eq!(
        response.headers().get(header::CONTENT_TYPE),
        Some(&header::HeaderValue::from_static("application/x-ndjson"))
    );
}

fn test_event() -> CapabilityTaskEvent {
    CapabilityTaskEvent {
        task_id: "task-1".to_string(),
        event_kind: CapabilityTaskEventKind::Completed,
        sequence: 1,
        chunk_index: None,
        chunk_delta: None,
        usage_delta: None,
        final_usage: None,
        finish_reason: None,
        is_truncated_finish_reason: None,
        checkpoint_key: None,
        artifact_refs: Vec::new(),
        error: None,
        payload: None,
    }
}
