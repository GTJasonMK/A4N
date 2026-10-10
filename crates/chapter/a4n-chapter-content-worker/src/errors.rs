pub(super) use a4n_worker_contract::WorkerFailure;

pub(super) fn chapter_error(
    error: a4n_db::chapter_content::ChapterContentEditError,
) -> WorkerFailure {
    match error {
        a4n_db::chapter_content::ChapterContentEditError::InvalidArgument(message) => {
            WorkerFailure::invalid_request(message)
        }
        a4n_db::chapter_content::ChapterContentEditError::NotFound(message) => {
            WorkerFailure::not_found(message)
        }
        a4n_db::chapter_content::ChapterContentEditError::InvalidState { current, operation } => {
            WorkerFailure::conflict(format!("{current}: {operation}"))
        }
        a4n_db::chapter_content::ChapterContentEditError::Database(error) => {
            WorkerFailure::unavailable(error.to_string())
        }
    }
}

pub(super) fn import_error(
    error: a4n_db::chapter_content::ChapterContentImportError,
) -> WorkerFailure {
    match error {
        a4n_db::chapter_content::ChapterContentImportError::InvalidArgument(message) => {
            WorkerFailure::invalid_request(message)
        }
        a4n_db::chapter_content::ChapterContentImportError::TargetChapterGenerating {
            chapter_number,
            status,
            operation,
        } => WorkerFailure::conflict(format!(
            "第 {chapter_number} 章正在生成中，请等待当前生成完成后再{operation}（当前状态：{status}）"
        )),
        a4n_db::chapter_content::ChapterContentImportError::Database(error) => {
            WorkerFailure::unavailable(error.to_string())
        }
    }
}

pub(super) fn project_error(error: a4n_db::project::ProjectMutationError) -> WorkerFailure {
    a4n_worker_db_adapter::map_project_mutation_error(error)
}

pub(super) fn workflow_error(error: a4n_db::workflow::WorkflowReadError) -> WorkerFailure {
    a4n_worker_db_adapter::map_workflow_read_error_as_conflict(error)
}
