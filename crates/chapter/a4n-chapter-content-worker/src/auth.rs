use super::WorkerState;
use a4n_worker_contract::WORKER_AUTH_HEADER;
use axum::http::HeaderMap;

impl WorkerState {
    pub(super) fn authorized(&self, headers: &HeaderMap) -> bool {
        a4n_worker_runtime::worker_token_authorized(
            headers,
            WORKER_AUTH_HEADER,
            self.auth_token.as_bytes(),
        )
    }
}
