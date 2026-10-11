use a4n_error::AppError;
use a4n_service_client::sidecar_response_payload;
use a4n_service_protocol::{
    CapabilityTaskEvent, CapabilityTaskEventKind, ServiceError, ServiceErrorCode, ServiceResponse,
};
use serde::de::DeserializeOwned;
use serde_json::Value;
use std::future::Future;
use tokio::sync::mpsc::Receiver;

type JsonResponse = ServiceResponse<Value>;

pub(super) enum GenerationEvent<T> {
    Progress(String),
    Completed(T),
}

#[derive(Debug, PartialEq)]
pub(super) enum GenerationStreamError {
    Transport(ServiceError),
    Event(String),
    Progress(String),
}

pub(super) fn parse_generation_event<T: DeserializeOwned>(
    event: CapabilityTaskEvent,
    service: &str,
) -> Result<GenerationEvent<T>, String> {
    match event.event_kind {
        CapabilityTaskEventKind::Progress => {
            let payload = event
                .payload
                .ok_or_else(|| format!("{service} sidecar progress 事件缺少 payload"))?;
            let message = payload
                .get("message")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|message| !message.is_empty())
                .ok_or_else(|| format!("{service} sidecar progress 事件缺少 message"))?;
            Ok(GenerationEvent::Progress(message.to_string()))
        }
        CapabilityTaskEventKind::Completed => {
            let payload = event
                .payload
                .ok_or_else(|| format!("{service} sidecar completed 事件缺少 payload"))?;
            serde_json::from_value(payload)
                .map(GenerationEvent::Completed)
                .map_err(|error| format!("{service} sidecar completed payload 解析失败: {error}"))
        }
        CapabilityTaskEventKind::Failed => {
            let error = event
                .error
                .ok_or_else(|| format!("{service} sidecar failed 事件缺少 error"))?;
            Err(service_error_string(error))
        }
        CapabilityTaskEventKind::Chunk => Err(format!(
            "{service} sidecar generation 事件类型不符合合同: chunk"
        )),
    }
}

pub(super) async fn collect_generation_output<T, F, P>(
    request: F,
    mut event_rx: Receiver<CapabilityTaskEvent>,
    service: &str,
    mut report_progress: P,
) -> Result<T, GenerationStreamError>
where
    T: DeserializeOwned,
    F: Future<Output = Result<(), ServiceError>>,
    P: FnMut(String) -> Result<(), String>,
{
    let mut request_future = Box::pin(request);
    let mut transport_complete = false;
    loop {
        tokio::select! {
            result = &mut request_future, if !transport_complete => {
                result.map_err(GenerationStreamError::Transport)?;
                transport_complete = true;
            }
            event = event_rx.recv() => {
                let event = event.ok_or_else(|| {
                    GenerationStreamError::Event(format!(
                        "{service} sidecar 生成事件流结束但缺少终止事件"
                    ))
                })?;
                match parse_generation_event::<T>(event, service)
                    .map_err(GenerationStreamError::Event)?
                {
                    GenerationEvent::Progress(message) => {
                        report_progress(message).map_err(GenerationStreamError::Progress)?;
                    }
                    GenerationEvent::Completed(output) => return Ok(output),
                }
            }
        }
    }
}

pub(crate) fn response_payload<T: DeserializeOwned>(
    response: JsonResponse,
    service: &str,
) -> Result<T, String> {
    serde_json::from_value(typed_response_payload(response, service)?)
        .map_err(|err| format!("{service} sidecar 响应 payload 解析失败: {err}"))
}

pub(crate) fn typed_response_payload<T>(
    response: ServiceResponse<T>,
    service: &str,
) -> Result<T, String> {
    sidecar_response_payload(response, service).map_err(service_error_string)
}

pub(super) fn service_error_string(error: a4n_service_protocol::ServiceError) -> String {
    if error.code == ServiceErrorCode::Cancelled {
        AppError::Cancelled.to_string()
    } else {
        error.message
    }
}

pub(super) fn service_app_error(message: impl Into<String>) -> AppError {
    AppError::ConfigError(message.into())
}

pub(super) fn service_error_app_error(error: a4n_service_protocol::ServiceError) -> AppError {
    if error.code == ServiceErrorCode::Cancelled {
        AppError::Cancelled
    } else {
        AppError::ConfigError(error.message)
    }
}

#[cfg(test)]
mod generation_stream_tests {
    use super::*;
    use a4n_service_protocol::CapabilityTaskEventKind;
    use serde_json::json;
    use tokio::sync::mpsc;

    fn event(
        event_kind: CapabilityTaskEventKind,
        sequence: u64,
        payload: Option<Value>,
    ) -> CapabilityTaskEvent {
        CapabilityTaskEvent {
            task_id: "video-task".to_string(),
            event_kind,
            sequence,
            error: None,
            payload,
        }
    }

    #[tokio::test]
    async fn collect_generation_output_forwards_progress_before_completion() {
        let (sender, event_rx) = mpsc::channel(2);
        sender
            .send(CapabilityTaskEvent::progress_message(
                "video-task".to_string(),
                1,
                "rendering",
            ))
            .await
            .expect("progress event queues");
        sender
            .send(event(
                CapabilityTaskEventKind::Completed,
                2,
                Some(json!({"result": "done"})),
            ))
            .await
            .expect("completion event queues");

        let mut progress = Vec::new();
        let output = collect_generation_output::<Value, _, _>(
            async { Ok::<(), ServiceError>(()) },
            event_rx,
            "image",
            |message| {
                progress.push(message);
                Ok(())
            },
        )
        .await
        .expect("completion payload is returned");

        assert_eq!(output, json!({"result": "done"}));
        assert_eq!(progress, ["rendering"]);
    }

    #[tokio::test]
    async fn collect_generation_output_rejects_stream_close_without_completion() {
        let (sender, event_rx) = mpsc::channel(1);
        drop(sender);

        let error = collect_generation_output::<Value, _, _>(
            async { Ok::<(), ServiceError>(()) },
            event_rx,
            "image",
            |_| Ok(()),
        )
        .await
        .expect_err("closed stream without completion must fail");

        assert!(matches!(
            error,
            GenerationStreamError::Event(message)
                if message == "image sidecar 生成事件流结束但缺少终止事件"
        ));
    }

    #[tokio::test]
    async fn collect_generation_output_preserves_transport_error() {
        let (_sender, event_rx) = mpsc::channel(1);

        let error = collect_generation_output::<Value, _, _>(
            async {
                Err(ServiceError::non_retryable(
                    ServiceErrorCode::ServiceUnavailable,
                    "unavailable",
                ))
            },
            event_rx,
            "video",
            |_| Ok(()),
        )
        .await
        .expect_err("transport error must propagate");

        assert!(matches!(
            error,
            GenerationStreamError::Transport(error)
                if error.code == ServiceErrorCode::ServiceUnavailable
        ));
    }

    #[tokio::test]
    async fn collect_generation_output_preserves_progress_sink_error() {
        let (sender, event_rx) = mpsc::channel(1);
        sender
            .send(CapabilityTaskEvent::progress_message(
                "video-task".to_string(),
                1,
                "rendering",
            ))
            .await
            .expect("progress event queues");

        let error = collect_generation_output::<Value, _, _>(
            async { Ok::<(), ServiceError>(()) },
            event_rx,
            "video",
            |message| Err(format!("sink rejected {message}")),
        )
        .await
        .expect_err("progress sink error must propagate");

        assert!(matches!(
            error,
            GenerationStreamError::Progress(message) if message == "sink rejected rendering"
        ));
    }
}
