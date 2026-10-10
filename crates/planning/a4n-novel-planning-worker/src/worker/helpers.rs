use super::errors::{collaboration_error, project_family_error, source_error};
use crate::*;

pub(super) async fn verify_family_permission(
    db: &DatabaseConnection,
    project_id: i64,
    user_id: i64,
    required: ProjectAccessRole,
) -> Result<(), WorkerFailure> {
    let (project_id, root_project_id) =
        a4n_db::project::project_or_derived_family_access_ids(db, project_id)
            .await
            .map_err(project_family_error)?;
    if let Some(root_project_id) = root_project_id {
        a4n_db::collaboration::verify_project_permissions(
            db,
            &[project_id, root_project_id],
            user_id,
            required,
        )
        .await
        .map_err(collaboration_error)?;
    } else {
        a4n_db::collaboration::verify_project_permission(db, project_id, user_id, required)
            .await
            .map_err(collaboration_error)?;
    }
    Ok(())
}

pub(super) fn validate_request<T>(
    request: &WorkerEnvelope<T>,
    operation: &str,
) -> Result<(), WorkerFailure> {
    request.validate_worker_command(operation, NOVEL_PLANNING_WORKER_ID, "novel-planning worker")
}

pub(super) fn validate_project_scope<T>(
    request: &WorkerEnvelope<T>,
    project_id: i64,
) -> Result<(), WorkerFailure> {
    if request.project_id != Some(project_id) {
        return Err(WorkerFailure::invalid_request(format!(
            "worker scope project_id 与 payload 不一致: scope={:?}, payload={project_id}",
            request.project_id
        )));
    }
    Ok(())
}

pub(super) fn validate_target_revision<T>(
    request: &WorkerEnvelope<T>,
    expected_revision: i32,
) -> Result<(), WorkerFailure> {
    if request.target_revision != Some(expected_revision as u64) {
        return Err(WorkerFailure::invalid_request(format!(
            "worker target_revision 与 payload 不一致: target={:?}, payload={expected_revision}",
            request.target_revision
        )));
    }
    Ok(())
}

pub(super) fn validate_optional_target_revision<T>(
    request: &WorkerEnvelope<T>,
    expected_revision: Option<i32>,
) -> Result<(), WorkerFailure> {
    match expected_revision {
        Some(expected_revision) => validate_target_revision(request, expected_revision),
        None if request.target_revision.is_some() => Err(WorkerFailure::invalid_request(
            "新建拍摄剧本不能携带 target_revision",
        )),
        None => Ok(()),
    }
}

pub(super) async fn resolve_source(
    state: &WorkerState,
    project_id: i64,
    source_chapter_id: i64,
) -> Result<a4n_domain::chapter::ResolvedChapterSource, WorkerFailure> {
    a4n_db::project::resolve_video_chapter_source(
        state.database.connection(),
        project_id,
        source_chapter_id,
    )
    .await
    .map_err(source_error)
}
