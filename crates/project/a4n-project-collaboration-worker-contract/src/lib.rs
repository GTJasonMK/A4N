//! Typed project membership and invitation commands owned by one worker.

pub use a4n_api_contract::collaboration::{
    ProjectInviteSummary, ProjectMemberRole, ProjectMemberSummary,
};
pub use a4n_worker_contract::{WorkerEnvelope, WorkerResponse};

use serde::{Deserialize, Serialize};

pub const PROJECT_COLLABORATION_WORKER_ID: &str = "project-collaboration-worker";
pub const PROJECT_COLLABORATION_WORKER_CONTRACT_VERSION: &str = "project-collaboration-v2";
pub const PROJECT_COLLABORATION_LIST_MEMBERS_OPERATION: &str = "project.collaboration.members.list";
pub const PROJECT_COLLABORATION_INVITE_MEMBER_OPERATION: &str =
    "project.collaboration.members.invite";
pub const PROJECT_COLLABORATION_LIST_INVITES_OPERATION: &str = "project.collaboration.invites.list";
pub const PROJECT_COLLABORATION_LIST_MY_INVITES_OPERATION: &str =
    "project.collaboration.invites.list_mine";
pub const PROJECT_COLLABORATION_ACCEPT_INVITE_OPERATION: &str =
    "project.collaboration.invites.accept";
pub const PROJECT_COLLABORATION_DECLINE_INVITE_OPERATION: &str =
    "project.collaboration.invites.decline";
pub const PROJECT_COLLABORATION_REVOKE_INVITE_OPERATION: &str =
    "project.collaboration.invites.revoke";
pub const PROJECT_COLLABORATION_UPDATE_ROLE_OPERATION: &str =
    "project.collaboration.members.update_role";
pub const PROJECT_COLLABORATION_REMOVE_MEMBER_OPERATION: &str =
    "project.collaboration.members.remove";
pub const PROJECT_COLLABORATION_LEAVE_OPERATION: &str = "project.collaboration.project.leave";
pub const PROJECT_COLLABORATION_CAPABILITY: &str = "project.collaboration";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ProjectInput {
    pub project_id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct InviteMemberInput {
    pub project_id: i64,
    pub username_or_email: String,
    pub role: ProjectMemberRole,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ProjectInviteInput {
    pub project_id: i64,
    pub invite_id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct InviteInput {
    pub invite_id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct MemberInput {
    pub project_id: i64,
    pub member_user_id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct UpdateMemberRoleInput {
    pub project_id: i64,
    pub member_user_id: i64,
    pub role: ProjectMemberRole,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct EmptyInput {}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProjectCollaborationAck {}

#[cfg(test)]
mod tests {
    use super::*;
    use a4n_worker_contract::{
        WORKER_SCHEMA_VERSION, WorkerLease, WorkerPrincipal, WorkerPrincipalKind,
    };

    #[test]
    fn project_command_scope_is_stable() {
        let request = WorkerEnvelope {
            schema_version: WORKER_SCHEMA_VERSION,
            request_id: "request".into(),
            trace_id: "trace".into(),
            principal: WorkerPrincipal {
                kind: WorkerPrincipalKind::User,
                subject: "7".into(),
            },
            operation_id: PROJECT_COLLABORATION_LIST_MEMBERS_OPERATION.into(),
            idempotency_key: "key".into(),
            project_id: Some(9),
            root_project_id: None,
            target_revision: None,
            runtime_config_snapshot_ref: None,
            lease: WorkerLease {
                worker_id: PROJECT_COLLABORATION_WORKER_ID.into(),
                epoch: 1,
                fence_token: "control".into(),
                expires_at: "9999-12-31T23:59:59Z".into(),
            },
            payload: ProjectInput { project_id: 9 },
        };
        request.validate().expect("project collaboration validates");
    }
}
