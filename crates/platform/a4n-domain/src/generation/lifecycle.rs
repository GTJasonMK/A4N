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
