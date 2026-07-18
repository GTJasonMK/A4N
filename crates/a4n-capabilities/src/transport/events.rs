use a4n_service_protocol::{
    ServiceError, ServiceErrorCode, ServiceTaskEvent, sanitize_service_task_event,
};
use futures::StreamExt;

use super::HttpSidecarTransport;
use super::errors::transport_error;
use crate::contracts::{CapabilityResult, JsonRequest};

impl HttpSidecarTransport {
    pub(crate) async fn post_event_stream(
        &self,
        path: &str,
        request: JsonRequest,
        events: tokio::sync::mpsc::UnboundedSender<ServiceTaskEvent>,
    ) -> CapabilityResult<()> {
        let mut stream = self
            .client
            .post(self.url(path))
            .json(&request)
            .send()
            .await
            .map_err(transport_error)?
            .error_for_status()
            .map_err(transport_error)?
            .bytes_stream();

        let mut buffer = Vec::<u8>::new();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(transport_error)?;
            buffer.extend_from_slice(&chunk);
            if drain_event_lines(&mut buffer, &events)? {
                return Ok(());
            }
        }
        if !buffer.is_empty() && !buffer.iter().all(u8::is_ascii_whitespace) {
            parse_event_line(&buffer).and_then(|event| send_event(&events, event))?;
        }
        Ok(())
    }
}

pub(crate) fn drain_event_lines(
    buffer: &mut Vec<u8>,
    events: &tokio::sync::mpsc::UnboundedSender<ServiceTaskEvent>,
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
        if parse_event_line(&line).and_then(|event| send_event(events, event))? {
            buffer.clear();
            return Ok(true);
        }
    }
    Ok(false)
}

pub(crate) fn parse_event_line(line: &[u8]) -> CapabilityResult<ServiceTaskEvent> {
    let mut event: ServiceTaskEvent =
        serde_json::from_slice(line).map_err(|error| ServiceError {
            code: ServiceErrorCode::Internal,
            message: format!("sidecar event JSON 解析失败: {error}"),
            retryable: Some(false),
        })?;
    sanitize_service_task_event(&mut event).map_err(|error| ServiceError {
        code: ServiceErrorCode::Internal,
        message: format!("sidecar event artifact ref 非法: {error}"),
        retryable: Some(false),
    })?;
    Ok(event)
}

fn send_event(
    events: &tokio::sync::mpsc::UnboundedSender<ServiceTaskEvent>,
    event: ServiceTaskEvent,
) -> CapabilityResult<bool> {
    let is_terminal = event.event_kind.is_terminal();
    events.send(event).map_err(|_| ServiceError {
        code: ServiceErrorCode::Cancelled,
        message: "sidecar event receiver 已关闭".to_string(),
        retryable: Some(false),
    })?;
    Ok(is_terminal)
}
