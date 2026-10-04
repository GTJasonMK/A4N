use a4n_db_models::models::project::project_member;
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, QueryOrder, Set};

mod types;
pub use types::{
    AccessibleProjectListInput, AccessibleProjectRecord, AccessibleProjectsPage,
    CollaborationDbError, CollaborationRootProject, ProjectAccessInfo, ProjectAccessInfoQuery,
    ProjectAccessRole, ProjectAccessScope, ProjectInviteSummary, ProjectMemberRole,
    ProjectMemberSummary,
};
use types::{
    INVITE_STATUS_ACCEPTED, INVITE_STATUS_DECLINED, INVITE_STATUS_EXPIRED, INVITE_STATUS_PENDING,
    INVITE_STATUS_REVOKED, INVITE_TTL_DAYS,
};

mod queries;
use queries::*;

mod access;
pub use access::{
    list_accessible_projects, project_access_info, project_access_info_batch,
    project_ids_with_permission, resolve_root_project, verify_project_permission,
    verify_project_permissions,
};

mod members;
pub use members::{
    leave_project, list_project_members, remove_project_member, update_project_member_role,
};

mod invites;
pub use invites::{
    accept_project_invite, decline_project_invite, invite_project_member, list_my_project_invites,
    list_project_invites, revoke_project_invite,
};

#[cfg(test)]
mod tests;
