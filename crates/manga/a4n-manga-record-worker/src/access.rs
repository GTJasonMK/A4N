use a4n_db::DbHandle;
use a4n_domain::collaboration::ProjectAccessRole;
use a4n_worker_client::WORKER_AUTH_HEADER;
use a4n_worker_contract::{WorkerEnvelope, WorkerPrincipalKind};
use axum::http::HeaderMap;
use subtle::ConstantTimeEq;

use super::{WorkerState, protocol::WorkerFailure};

impl WorkerState {
    pub(super) fn authorized(&self, headers: &HeaderMap) -> bool {
        headers
            .get(WORKER_AUTH_HEADER)
            .is_some_and(|value| bool::from(value.as_bytes().ct_eq(self.auth_token.as_bytes())))
    }
}

pub(super) async fn verify_manga_viewer(
    database: &DbHandle,
    project_id: i64,
    user_id: i64,
) -> Result<(), WorkerFailure> {
    let (project_id, root_project_id) =
        a4n_db::project::project_or_derived_family_access_ids(database.connection(), project_id)
            .await
            .map_err(super::protocol::project_family_error)?;
    let project_ids = root_project_id
        .into_iter()
        .chain([project_id])
        .collect::<Vec<_>>();
    a4n_db::collaboration::verify_project_permissions(
        database.connection(),
        &project_ids,
        user_id,
        ProjectAccessRole::Viewer,
    )
    .await
    .map(|_| ())
    .map_err(super::protocol::collaboration_error)
}

pub(super) async fn verify_project_permission(
    database: &DbHandle,
    project_id: i64,
    user_id: i64,
    role: ProjectAccessRole,
) -> Result<(), WorkerFailure> {
    a4n_db::collaboration::verify_project_permission(
        database.connection(),
        project_id,
        user_id,
        role,
    )
    .await
    .map(|_| ())
    .map_err(super::protocol::collaboration_error)
}

pub(super) fn user_id<T: serde::Serialize>(
    request: &WorkerEnvelope<T>,
) -> Result<i64, WorkerFailure> {
    if request.principal.kind != WorkerPrincipalKind::User {
        return Err(WorkerFailure::unauthorized("仅允许用户 principal"));
    }
    request
        .principal
        .subject
        .parse::<i64>()
        .ok()
        .filter(|id| *id > 0)
        .ok_or_else(|| WorkerFailure::unauthorized("用户 principal subject 无效"))
}
