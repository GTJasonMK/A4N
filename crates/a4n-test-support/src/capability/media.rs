use std::sync::Mutex;

use a4n_service_client::ServiceClientResult as CapabilityResult;
use a4n_service_protocol::{CapabilityTaskEvent, ServiceError, ServiceErrorCode, ServiceRequest};

use super::unexpected_call;

type JsonRequest = ServiceRequest<serde_json::Value>;

pub(super) struct RequestLedger<T = JsonRequest> {
    requests: Mutex<Vec<T>>,
}

impl<T> Default for RequestLedger<T> {
    fn default() -> Self {
        Self {
            requests: Mutex::new(Vec::new()),
        }
    }
}

impl<T: Clone> RequestLedger<T> {
    pub(super) fn record(&self, request: T) {
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

    pub(super) fn last(&self) -> Option<T> {
        self.requests
            .lock()
            .expect("recording media request lock should not be poisoned")
            .last()
            .cloned()
    }
}

pub(super) async fn send_configured_events(
    client: &str,
    operation: &str,
    configured: Option<&[CapabilityTaskEvent]>,
    request_id: &str,
    events: &tokio::sync::mpsc::Sender<CapabilityTaskEvent>,
) -> CapabilityResult<()> {
    let configured = configured.ok_or_else(|| unexpected_call(client, operation))?;
    for mut event in configured.iter().cloned() {
        event.task_id = request_id.to_string();
        events.send(event).await.map_err(|_| {
            ServiceError::non_retryable(ServiceErrorCode::Cancelled, "event receiver closed")
        })?;
    }
    Ok(())
}
