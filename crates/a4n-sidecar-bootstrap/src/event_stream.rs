use std::convert::Infallible;

use a4n_service_protocol::{
    CapabilityTaskEvent, ServiceError, ServiceErrorCode, redact_sensitive_message,
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
    CapabilityTaskEvent::failed(
        "service-event-serialization".to_string(),
        0,
        ServiceError::non_retryable(
            ServiceErrorCode::Internal,
            redact_sensitive_message(format!("service task event serialization failed: {error}")),
        ),
    )
}

#[cfg(test)]
mod tests;
