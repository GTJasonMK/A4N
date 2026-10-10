use super::errors::WorkerFailure;
use a4n_db::DbHandle;
use a4n_domain::collaboration::ProjectAccessRole;

pub(super) async fn verify_project_editor(
    database: &DbHandle,
    project_id: i64,
    user_id: i64,
) -> Result<(), WorkerFailure> {
    verify_project_permission(database, project_id, user_id, ProjectAccessRole::Editor).await
}

pub(super) async fn verify_project_viewer(
    database: &DbHandle,
    project_id: i64,
    user_id: i64,
) -> Result<(), WorkerFailure> {
    verify_project_permission(database, project_id, user_id, ProjectAccessRole::Viewer).await
}

async fn verify_project_permission(
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
    .map_err(a4n_worker_db_adapter::map_collaboration_db_error)
}
