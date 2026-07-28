use std::convert::Infallible;

use a4n_service_protocol::{
    CapabilityTaskEvent, CapabilityTaskEventKind, ServiceError, ServiceErrorCode,
    redact_sensitive_message,
};
use axum::{
    body::{Body, Bytes},
    http::header,
    response::{IntoResponse, Response},
};
use futures::{Stream, stream};

const SERIALIZATION_FAILURE_EVENT: &[u8] = br#"{"task_id":"service-event-serialization","event_kind":"failed","sequence":0,"error":{"code":"internal","message":"service task event serialization failed","retryable":false}}"#;

pub fn service_event_line(event: CapabilityTaskEvent) -> Bytes {
    let mut line = match serde_json::to_vec(&event) {
        Ok(line) => line,
        Err(error) => {
            let failed_event = serialization_failed_event(error);
            match serde_json::to_vec(&failed_event) {
                Ok(line) => line,
                Err(_) => SERIALIZATION_FAILURE_EVENT.to_vec(),
            }
        }
    };
    line.push(b'\n');
    Bytes::from(line)
}

pub fn single_service_event_stream(
    event: CapabilityTaskEvent,
) -> impl Stream<Item = Result<Bytes, Infallible>> + Send + 'static {
    stream::once(async move { Ok(service_event_line(event)) })
}

pub fn service_event_ndjson_response(
    stream: impl Stream<Item = Result<Bytes, Infallible>> + Send + 'static,
    event_context: &str,
) -> Response {
    match Response::builder()
        .header(header::CONTENT_TYPE, "application/x-ndjson")
        .body(Body::from_stream(stream))
    {
        Ok(response) => response,
        Err(error) => (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            redact_sensitive_message(format!(
                "failed to build {event_context} event response: {error}"
            )),
        )
            .into_response(),
    }
}

fn serialization_failed_event(error: serde_json::Error) -> CapabilityTaskEvent {
    CapabilityTaskEvent {
        task_id: "service-event-serialization".to_string(),
        event_kind: CapabilityTaskEventKind::Failed,
        sequence: 0,
        chunk_index: None,
        chunk_delta: None,
        usage_delta: None,
        final_usage: None,
        finish_reason: None,
        is_truncated_finish_reason: None,
        checkpoint_key: None,
        artifact_refs: Vec::new(),
        error: Some(ServiceError {
            code: ServiceErrorCode::Internal,
            message: redact_sensitive_message(format!(
                "service task event serialization failed: {error}"
            )),
            retryable: Some(false),
        }),
        payload: None,
    }
}

#[cfg(test)]
mod tests;
