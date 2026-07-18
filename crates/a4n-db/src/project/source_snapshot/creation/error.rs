use crate::{project::ProjectMutationError, text_anchor::TextAnchorDbError};

impl From<TextAnchorDbError> for ProjectMutationError {
    fn from(error: TextAnchorDbError) -> Self {
        match error {
            TextAnchorDbError::Database(error) => Self::Database(error),
            TextAnchorDbError::NotFound(message) => Self::NotFound(message),
            TextAnchorDbError::InvalidArgument(message) => Self::InvalidArgument(message),
        }
    }
}
