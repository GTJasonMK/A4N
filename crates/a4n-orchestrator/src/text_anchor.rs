pub(crate) use a4n_db::text_anchor::{EVIDENCE_ROLE_FIRST_SEEN, SOURCE_KIND_CHAPTER_CONTENT};

pub(crate) fn text_anchor_db_error(
    error: a4n_db::text_anchor::TextAnchorDbError,
) -> a4n_error::AppError {
    match error {
        a4n_db::text_anchor::TextAnchorDbError::Database(error) => {
            a4n_error::AppError::Database(error)
        }
        a4n_db::text_anchor::TextAnchorDbError::NotFound(message) => {
            a4n_error::AppError::NotFound(message)
        }
        a4n_db::text_anchor::TextAnchorDbError::InvalidArgument(message) => {
            a4n_error::AppError::InvalidArgument(message)
        }
    }
}
