use std::sync::Mutex;

use a4n_capabilities::{CapabilityResult, JsonRequest};
use a4n_service_protocol::{ServiceError, ServiceErrorCode, ServiceTaskEvent};

use super::unexpected_call;

#[derive(Default)]
pub(super) struct RequestLedger {
    requests: Mutex<Vec<JsonRequest>>,
}

impl RequestLedger {
    pub(super) fn record(&self, request: JsonRequest) {
        self.requests
            .lock()
            .expect("recording media request lock should not be poisoned")
            .push(request);
    }

    pub(super) fn count(&self) -> usize {
        self.requests
            .lock()
            .expect("recording media request lock should not be poisoned")
            .len()
    }

    pub(super) fn last(&self) -> Option<JsonRequest> {
        self.requests
            .lock()
            .expect("recording media request lock should not be poisoned")
            .last()
            .cloned()
    }
}

pub(super) fn send_configured_events(
    client: &str,
    operation: &str,
    configured: Option<&[ServiceTaskEvent]>,
    request_id: &str,
    events: &tokio::sync::mpsc::UnboundedSender<ServiceTaskEvent>,
) -> CapabilityResult<()> {
    let configured = configured.ok_or_else(|| unexpected_call(client, operation))?;
    for mut event in configured.iter().cloned() {
        event.task_id = request_id.to_string();
        events.send(event).map_err(|_| ServiceError {
            code: ServiceErrorCode::Cancelled,
            message: "event receiver closed".to_string(),
            retryable: Some(false),
        })?;
    }
    Ok(())
}
