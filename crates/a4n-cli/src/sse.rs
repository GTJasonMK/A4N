use anyhow::{Context, Result, anyhow};
use futures::StreamExt;
use reqwest::Response;
use serde_json::Value;

pub(crate) enum StreamOutcome {
    Terminal(Value),
    Disconnected {
        last_sequence: Option<u64>,
        reason: Option<String>,
    },
}

pub(crate) async fn consume_generation_events(
    response: Response,
    mut emit: impl FnMut(&Value) -> Result<()>,
) -> Result<StreamOutcome> {
    consume_events(response, &mut emit, true, "generation").await
}

pub(crate) async fn consume_agent_events(
    response: Response,
    mut emit: impl FnMut(&Value) -> Result<()>,
) -> Result<StreamOutcome> {
    consume_events(response, &mut emit, false, "Agent").await
}

async fn consume_events(
    response: Response,
    emit: &mut impl FnMut(&Value) -> Result<()>,
    stop_at_terminal: bool,
    stream_label: &str,
) -> Result<StreamOutcome> {
    let mut stream = response.bytes_stream();
    let mut buffer = Vec::new();
    let mut last_sequence = None;

    while let Some(chunk) = stream.next().await {
        let chunk = match chunk {
            Ok(chunk) => chunk,
            Err(error) => {
                return Ok(StreamOutcome::Disconnected {
                    last_sequence,
                    reason: Some(error.without_url().to_string()),
                });
            }
        };
        buffer.extend_from_slice(&chunk);
        while let Some((frame_end, delimiter_len)) = frame_boundary(&buffer) {
            let frame = buffer.drain(..frame_end).collect::<Vec<_>>();
            buffer.drain(..delimiter_len);
            let Some(event) = parse_frame(&frame)? else {
                continue;
            };
            last_sequence = Some(
                event
                    .get("sequence")
                    .and_then(Value::as_u64)
                    .ok_or_else(|| anyhow!("{stream_label} SSE event 缺少有效 sequence"))?,
            );
            emit(&event)?;
            if stop_at_terminal && is_terminal(&event) {
                return Ok(StreamOutcome::Terminal(event));
            }
        }
    }

    Ok(StreamOutcome::Disconnected {
        last_sequence,
        reason: None,
    })
}

fn frame_boundary(buffer: &[u8]) -> Option<(usize, usize)> {
    let lf = buffer.windows(2).position(|window| window == b"\n\n");
    let crlf = buffer.windows(4).position(|window| window == b"\r\n\r\n");
    match (lf, crlf) {
        (Some(lf), Some(crlf)) if lf <= crlf => Some((lf, 2)),
        (Some(_), Some(crlf)) => Some((crlf, 4)),
        (Some(lf), None) => Some((lf, 2)),
        (None, Some(crlf)) => Some((crlf, 4)),
        (None, None) => None,
    }
}

fn parse_frame(frame: &[u8]) -> Result<Option<Value>> {
    let frame = std::str::from_utf8(frame).context("SSE frame 不是有效 UTF-8")?;
    let data = frame
        .lines()
        .filter_map(|line| line.strip_prefix("data:"))
        .map(|value| value.strip_prefix(' ').unwrap_or(value))
        .collect::<Vec<_>>()
        .join("\n");
    if data.is_empty() {
        return Ok(None);
    }
    serde_json::from_str(&data)
        .map(Some)
        .with_context(|| format!("generation SSE data 不是有效 JSON: {data}"))
}

fn is_terminal(event: &Value) -> bool {
    event
        .pointer("/event/kind")
        .and_then(Value::as_str)
        .is_some_and(|kind| matches!(kind, "completed" | "failed" | "cancelled" | "interrupted"))
}

#[cfg(test)]
mod tests {
    use super::{frame_boundary, is_terminal, parse_frame};

    #[test]
    fn parses_generation_event_and_ignores_keep_alive() {
        assert!(parse_frame(b": keep-alive").unwrap().is_none());
        let value = parse_frame(
            br#"event: generation_task_event
data: {"event":{"kind":"completed"},"sequence":3}"#,
        )
        .unwrap()
        .unwrap();
        assert!(is_terminal(&value));
    }

    #[test]
    fn detects_both_sse_delimiters() {
        assert_eq!(frame_boundary(b"data: {}\n\nrest"), Some((8, 2)));
        assert_eq!(frame_boundary(b"data: {}\r\n\r\nrest"), Some((8, 4)));
    }
}
