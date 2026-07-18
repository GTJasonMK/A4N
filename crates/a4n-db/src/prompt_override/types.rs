use sea_orm::DbErr;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectOverrideIdentity {
    pub project_id: i64,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectOverrideHydration {
    pub identity: ProjectOverrideIdentity,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectPromptOverrideSummary {
    pub project_id: i64,
    pub prompt_name: String,
    pub content: Value,
    pub updated_at: String,
}

#[derive(Debug, thiserror::Error)]
pub enum PromptOverrideDbError {
    #[error("{0}")]
    InvalidArgument(String),
    #[error("{0}")]
    NotFound(String),
    #[error("{source}")]
    Project {
        #[source]
        source: crate::project::ProjectMutationError,
    },
    #[error("{operation}: {source}")]
    Database {
        operation: &'static str,
        #[source]
        source: DbErr,
    },
}

impl From<crate::project::ProjectMutationError> for PromptOverrideDbError {
    fn from(source: crate::project::ProjectMutationError) -> Self {
        Self::Project { source }
    }
}

pub(super) fn invalid_argument(message: impl Into<String>) -> PromptOverrideDbError {
    PromptOverrideDbError::InvalidArgument(message.into())
}

pub(super) fn database(operation: &'static str, source: DbErr) -> PromptOverrideDbError {
    PromptOverrideDbError::Database { operation, source }
}
