use crate::{
    project::ProjectMutationError, shooting_script::ShootingScriptDbError,
    text_anchor::TextAnchorDbError,
};

impl From<ShootingScriptDbError> for ProjectMutationError {
    fn from(error: ShootingScriptDbError) -> Self {
        match error {
            ShootingScriptDbError::Database(error) => Self::Database(error),
            other => Self::Internal(other.to_string()),
        }
    }
}

impl From<TextAnchorDbError> for ProjectMutationError {
    fn from(error: TextAnchorDbError) -> Self {
        match error {
            TextAnchorDbError::Database(error) => Self::Database(error),
            TextAnchorDbError::NotFound(message) => Self::NotFound(message),
            TextAnchorDbError::InvalidArgument(message) => Self::InvalidArgument(message),
        }
    }
}
