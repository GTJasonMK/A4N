pub use a4n_db::text_anchor::{
    CurrentChapterContentAnchorRows, EVIDENCE_ROLE_APPEARANCE, EVIDENCE_ROLE_BEHAVIOR,
    EVIDENCE_ROLE_FIRST_SEEN, EVIDENCE_ROLE_LOCATION, EVIDENCE_ROLE_RELATIONSHIP,
    EVIDENCE_ROLE_STATE_CHANGE, SOURCE_KIND_CHAPTER_CONTENT, TextAnchorEvidenceInput,
    TextAnchorProfileKind,
};

pub(crate) fn text_anchor_db_error(
    error: a4n_db::text_anchor::TextAnchorDbError,
) -> a4n_core::error::AppError {
    match error {
        a4n_db::text_anchor::TextAnchorDbError::Database(error) => {
            a4n_core::error::AppError::Database(error)
        }
        a4n_db::text_anchor::TextAnchorDbError::NotFound(message) => {
            a4n_core::error::AppError::NotFound(message)
        }
        a4n_db::text_anchor::TextAnchorDbError::InvalidArgument(message) => {
            a4n_core::error::AppError::InvalidArgument(message)
        }
    }
}
