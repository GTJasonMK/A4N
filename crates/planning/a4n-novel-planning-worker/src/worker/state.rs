mod mutations;
mod reads;

use crate::WorkerState;
use a4n_worker_contract::WORKER_AUTH_HEADER;
use axum::http::HeaderMap;
use subtle::ConstantTimeEq;

impl WorkerState {
    pub(super) fn authorized(&self, headers: &HeaderMap) -> bool {
        headers
            .get(WORKER_AUTH_HEADER)
            .is_some_and(|value| bool::from(value.as_bytes().ct_eq(self.auth_token.as_bytes())))
    }
}
