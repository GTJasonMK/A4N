use a4n_worker_contract::WORKER_AUTH_HEADER;
use a4n_worker_contract::{WorkerPrincipal, WorkerPrincipalKind};
use axum::http::HeaderMap;
use subtle::ConstantTimeEq;

use super::{WorkerState, protocol::WorkerFailure};

impl WorkerState {
    pub(super) fn authorized(&self, headers: &HeaderMap) -> bool {
        headers
            .get(WORKER_AUTH_HEADER)
            .and_then(|value| value.to_str().ok())
            .is_some_and(|value| value.as_bytes().ct_eq(self.auth_token.as_bytes()).into())
    }
}

pub(super) fn require_service_principal(principal: &WorkerPrincipal) -> Result<(), WorkerFailure> {
    if principal.kind != WorkerPrincipalKind::Service {
        return Err(WorkerFailure::unauthorized(
            "workflow lifecycle 仅允许 service principal",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use a4n_worker_contract::{WorkerPrincipal, WorkerPrincipalKind};

    use super::require_service_principal;

    #[test]
    fn lifecycle_rejects_user_principals() {
        let user = WorkerPrincipal {
            kind: WorkerPrincipalKind::User,
            subject: "7".into(),
        };
        let service = WorkerPrincipal {
            kind: WorkerPrincipalKind::Service,
            subject: "runtime".into(),
        };

        assert!(require_service_principal(&user).is_err());
        assert!(require_service_principal(&service).is_ok());
    }
}
