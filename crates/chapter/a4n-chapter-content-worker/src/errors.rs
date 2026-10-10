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
