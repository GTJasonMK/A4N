use a4n_service_protocol::{
    CapabilityTaskEvent, SIDECAR_EVENT_CONTENT_TYPE, ServiceError, ServiceErrorCode,
    sanitize_capability_task_event,
};
use futures::StreamExt;
use reqwest::header::CONTENT_TYPE;

use super::HttpSidecarTransport;
use super::errors::{timeout_error, transport_error};
use crate::contracts::{CapabilityResult, JsonRequest};

impl HttpSidecarTransport {
    pub(crate) async fn post_event_stream(
        &self,
        path: &str,
        request: JsonRequest,
        events: tokio::sync::mpsc::Sender<CapabilityTaskEvent>,
    ) -> CapabilityResult<()> {
        let client = self.request_client()?;
        let response = tokio::time::timeout(self.connect_timeout, async {
            self.authorize_request(client.post(self.url(path)), &request)?
                .json(&request)
                .send()
                .await
                .map_err(transport_error)?
                .error_for_status()
                .map_err(transport_error)
        })
        .await
        .map_err(|_| timeout_error("event stream connect"))??;
        if response
            .headers()
            .get(CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            != Some(SIDECAR_EVENT_CONTENT_TYPE)
        {
            return Err(invalid_event_stream(format!(
                "Content-Type 必须为 {SIDECAR_EVENT_CONTENT_TYPE}"
            )));
        }
        let mut stream = response.bytes_stream();

        let mut buffer = Vec::<u8>::new();
        loop {
            let next = tokio::select! {
                _ = events.closed() => return Err(cancelled_error()),
                next = tokio::time::timeout(self.stream_idle_timeout, stream.next()) => {
                    next.map_err(|_| timeout_error("event stream idle"))?
                }
            };
            let Some(chunk) = next else { break };
            let chunk = chunk.map_err(transport_error)?;
            buffer.extend_from_slice(&chunk);
            if drain_event_lines(&mut buffer, &events).await? {
                return Ok(());
            }
        }
        if buffer.iter().any(|byte| !byte.is_ascii_whitespace()) {
            return Err(invalid_event_stream("包含未以换行结束的 NDJSON 事件"));
        }
        Err(invalid_event_stream("在终态事件前结束"))
    }
}

pub(crate) async fn drain_event_lines(
    buffer: &mut Vec<u8>,
    events: &tokio::sync::mpsc::Sender<CapabilityTaskEvent>,
) -> CapabilityResult<bool> {
    let mut consumed = 0;
    while let Some(offset) = buffer[consumed..].iter().position(|byte| *byte == b'\n') {
        let newline = consumed + offset;
        let line_end = if newline > consumed && buffer[newline - 1] == b'\r' {
            newline - 1
        } else {
            newline
        };
        let event = parse_event_line(&buffer[consumed..line_end])?;
        consumed = newline + 1;
        if send_event(events, event).await? {
            buffer.clear();
            return Ok(true);
        }
    }
    if consumed > 0 {
        buffer.drain(..consumed);
    }
    Ok(false)
}

pub(crate) fn parse_event_line(line: &[u8]) -> CapabilityResult<CapabilityTaskEvent> {
    let mut event: CapabilityTaskEvent = serde_json::from_slice(line).map_err(|error| {
        ServiceError::non_retryable(
            ServiceErrorCode::Internal,
            format!("sidecar event JSON 解析失败: {error}"),
        )
    })?;
    sanitize_capability_task_event(&mut event);
    Ok(event)
}

async fn send_event(
    events: &tokio::sync::mpsc::Sender<CapabilityTaskEvent>,
    event: CapabilityTaskEvent,
) -> CapabilityResult<bool> {
    let is_terminal = event.event_kind.is_terminal();
    events.send(event).await.map_err(|_| cancelled_error())?;
    Ok(is_terminal)
}

fn cancelled_error() -> ServiceError {
    ServiceError::non_retryable(ServiceErrorCode::Cancelled, "sidecar event receiver 已关闭")
}

fn invalid_event_stream(message: impl std::fmt::Display) -> ServiceError {
    ServiceError::non_retryable(
        ServiceErrorCode::Internal,
        format!("sidecar event stream {message}"),
    )
}
