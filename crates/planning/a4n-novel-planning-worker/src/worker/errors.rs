use crate::*;

pub(super) fn source_error(error: a4n_db::project::ResolveChapterSourceError) -> WorkerFailure {
    match error {
        a4n_db::project::ResolveChapterSourceError::InvalidArgument(message) => {
            WorkerFailure::invalid_request(message)
        }
        a4n_db::project::ResolveChapterSourceError::InvalidState(message) => {
            WorkerFailure::conflict(message)
        }
        a4n_db::project::ResolveChapterSourceError::NotFound(message) => {
            WorkerFailure::not_found(message)
        }
        a4n_db::project::ResolveChapterSourceError::Database(error) => {
            WorkerFailure::unavailable(error.to_string())
        }
    }
}

pub(super) fn project_family_error(error: a4n_db::project::ProjectFamilyDbError) -> WorkerFailure {
    a4n_worker_db_adapter::map_project_family_db_error(error)
}

pub(super) fn collaboration_error(
    error: a4n_db::collaboration::CollaborationDbError,
) -> WorkerFailure {
    a4n_worker_db_adapter::map_collaboration_db_error(error)
}

pub(super) fn visual_plan_error(error: ChapterVisualPlanDbError) -> WorkerFailure {
    match error {
        ChapterVisualPlanDbError::InvalidArgument(message) => {
            WorkerFailure::invalid_request(message)
        }
        ChapterVisualPlanDbError::InvalidState(message) => WorkerFailure::conflict(message),
        ChapterVisualPlanDbError::NotFound(message) => WorkerFailure::not_found(message),
        ChapterVisualPlanDbError::RevisionConflict {
            record_id,
            expected_revision,
        } => WorkerFailure::conflict(format!(
            "chapter_visual_plan_revision_conflict:id={record_id},expected={expected_revision}"
        )),
        ChapterVisualPlanDbError::Database(error) => WorkerFailure::unavailable(error.to_string()),
    }
}

pub(crate) fn shooting_script_error(
    error: a4n_db::shooting_script::ShootingScriptDbError,
) -> WorkerFailure {
    match error {
        a4n_db::shooting_script::ShootingScriptDbError::InvalidArgument(message) => {
            WorkerFailure::invalid_request(message)
        }
        a4n_db::shooting_script::ShootingScriptDbError::InvalidState(message) => {
            WorkerFailure::conflict(message)
        }
        a4n_db::shooting_script::ShootingScriptDbError::NotFound(message) => {
            WorkerFailure::not_found(message)
        }
        a4n_db::shooting_script::ShootingScriptDbError::RevisionConflict {
            record_id,
            expected_revision,
        } => WorkerFailure::conflict(format!(
            "shooting_script_revision_conflict:id={record_id},expected={expected_revision}"
        )),
        a4n_db::shooting_script::ShootingScriptDbError::Database(error) => {
            WorkerFailure::unavailable(error.to_string())
        }
    }
}
