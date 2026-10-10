use a4n_db::DbHandle;
use a4n_domain::collaboration::ProjectAccessRole;
use a4n_worker_contract::WORKER_AUTH_HEADER;
use axum::http::HeaderMap;

use super::{WorkerState, protocol::WorkerFailure};

impl WorkerState {
    pub(super) fn authorized(&self, headers: &HeaderMap) -> bool {
        a4n_worker_runtime::worker_token_authorized(
            headers,
            WORKER_AUTH_HEADER,
            self.auth_token.as_bytes(),
        )
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
    .map_err(a4n_worker_db_adapter::map_collaboration_db_error)
}
