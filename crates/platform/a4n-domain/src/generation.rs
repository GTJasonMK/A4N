use serde::Serialize;
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenerationTaskListCursor {
    pub updated_at_ms: i64,
    pub task_id: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectLifecycleOperation {
    Delete,
    Reset,
}

impl ProjectLifecycleOperation {
    pub const fn gate(self) -> &'static str {
        match self {
            Self::Delete => "deleting",
            Self::Reset => "resetting",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LifecycleGateClaimState {
    Started,
    InProgress,
    Complete,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectLifecycleGateClaim {
    pub project_id: i64,
    pub user_id: i64,
    pub root_project_id: Option<i64>,
    pub operation: ProjectLifecycleOperation,
    pub state: LifecycleGateClaimState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserLifecycleGateClaim {
    pub user_id: i64,
    pub state: LifecycleGateClaimState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenerationLifecycleTask {
    pub task_id: String,
    pub attempt: u32,
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PendingLifecycleOperation {
    DisableUser {
        user_id: i64,
    },
    DeleteProject {
        project_id: i64,
        user_id: i64,
        root_project_id: Option<i64>,
    },
    ResetProject {
        project_id: i64,
        user_id: i64,
        root_project_id: Option<i64>,
    },
}

pub fn canonical_json_bytes<T>(value: &T) -> Result<Vec<u8>, serde_json::Error>
where
    T: Serialize,
{
    serde_json_canonicalizer::to_vec(value)
}

pub fn canonical_json_text<T>(value: &T) -> Result<String, serde_json::Error>
where
    T: Serialize,
{
    serde_json_canonicalizer::to_string(value)
}

pub fn canonical_json_sha256<T>(value: &T) -> Result<String, serde_json::Error>
where
    T: Serialize,
{
    let canonical = canonical_json_bytes(value)?;
    let mut hasher = Sha256::new();
    hasher.update(canonical);
    Ok(format!("{:x}", hasher.finalize()))
}
