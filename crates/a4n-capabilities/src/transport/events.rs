use a4n_service_protocol::{
    CapabilityTaskEvent, ServiceError, ServiceErrorCode, sanitize_capability_task_event,
};
use futures::StreamExt;

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
        let mut stream = tokio::time::timeout(self.connect_timeout, async {
            self.authorize_request(client.post(self.url(path)), &request)
                .json(&request)
                .send()
                .await
                .map_err(transport_error)?
                .error_for_status()
                .map_err(transport_error)
        })
        .await
        .map_err(|_| timeout_error("event stream connect"))??
        .bytes_stream();

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
        if !buffer.is_empty() && !buffer.iter().all(u8::is_ascii_whitespace) {
            send_event(&events, parse_event_line(&buffer)?).await?;
        }
        Ok(())
    }
}

pub(crate) async fn drain_event_lines(
    buffer: &mut Vec<u8>,
    events: &tokio::sync::mpsc::Sender<CapabilityTaskEvent>,
) -> CapabilityResult<bool> {
    while let Some(pos) = buffer.iter().position(|byte| *byte == b'\n') {
        let mut line = buffer.drain(..=pos).collect::<Vec<_>>();
        if line.last() == Some(&b'\n') {
            line.pop();
        }
        if line.last() == Some(&b'\r') {
            line.pop();
        }
        if line.iter().all(u8::is_ascii_whitespace) {
            continue;
        }
        if send_event(events, parse_event_line(&line)?).await? {
            buffer.clear();
            return Ok(true);
        }
    }
    Ok(false)
}

pub(crate) fn parse_event_line(line: &[u8]) -> CapabilityResult<CapabilityTaskEvent> {
    let mut event: CapabilityTaskEvent =
        serde_json::from_slice(line).map_err(|error| ServiceError {
            code: ServiceErrorCode::Internal,
            message: format!("sidecar event JSON 解析失败: {error}"),
            retryable: Some(false),
        })?;
    sanitize_capability_task_event(&mut event).map_err(|error| ServiceError {
        code: ServiceErrorCode::Internal,
        message: format!("sidecar event artifact ref 非法: {error}"),
        retryable: Some(false),
    })?;
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
    ServiceError {
        code: ServiceErrorCode::Cancelled,
        message: "sidecar event receiver 已关闭".to_string(),
        retryable: Some(false),
    }
}
