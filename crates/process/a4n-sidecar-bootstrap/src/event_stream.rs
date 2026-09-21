use std::convert::Infallible;

use a4n_service_protocol::{
    CapabilityTaskEvent, SIDECAR_EVENT_CONTENT_TYPE, SIDECAR_EVENT_QUEUE_CAPACITY, ServiceError,
};
use axum::{
    body::{Body, Bytes},
    http::header,
    response::Response,
};
use futures::{Stream, stream};

pub fn service_event_line(event: CapabilityTaskEvent) -> Bytes {
    let mut line = serde_json::to_vec(&event)
        .expect("CapabilityTaskEvent 仅包含 JSON 兼容字段，序列化不应失败");
    line.push(b'\n');
    Bytes::from(line)
}

pub fn service_event_channel() -> (
    tokio::sync::mpsc::Sender<CapabilityTaskEvent>,
    impl Stream<Item = Result<Bytes, Infallible>> + Send + 'static,
) {
    let (event_tx, event_rx) = tokio::sync::mpsc::channel(SIDECAR_EVENT_QUEUE_CAPACITY);
    let event_stream = stream::unfold(event_rx, |mut event_rx| async move {
        event_rx
            .recv()
            .await
            .map(|event| (Ok(service_event_line(event)), event_rx))
    });
    (event_tx, event_stream)
}

pub fn service_event_ndjson_response(
    stream: impl Stream<Item = Result<Bytes, Infallible>> + Send + 'static,
) -> Response {
    Response::builder()
        .header(header::CONTENT_TYPE, SIDECAR_EVENT_CONTENT_TYPE)
        .body(Body::from_stream(stream))
        .expect("静态 NDJSON Content-Type 必须是合法响应头")
}

pub fn failed_service_event_response(task_id: String, error: ServiceError) -> Response {
    let event = CapabilityTaskEvent::failed(task_id, 1, error);
    service_event_ndjson_response(stream::once(async move {
        Ok::<_, Infallible>(service_event_line(event))
    }))
}

#[cfg(test)]
mod tests;
