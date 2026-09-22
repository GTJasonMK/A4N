use a4n_db::chapter_content::ChapterGenerationLifecycleError;
use a4n_error::AppError;

mod completion;
mod session;
mod validation;

pub(in crate::chapter_generation) use completion::ChapterGenerationTrace;
pub(in crate::chapter_generation) use session::ChapterGenerationSession;

fn chapter_generation_lifecycle_error_to_app_error(
    error: ChapterGenerationLifecycleError,
) -> AppError {
    match error {
        ChapterGenerationLifecycleError::InvalidArgument(message) => {
            AppError::InvalidArgument(message)
        }
        ChapterGenerationLifecycleError::NotFound(message) => AppError::NotFound(message),
        ChapterGenerationLifecycleError::InvalidState { current, operation } => {
            AppError::InvalidState { current, operation }
        }
        ChapterGenerationLifecycleError::Database(error) => AppError::Database(error),
    }
}
