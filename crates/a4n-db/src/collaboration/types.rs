mod contracts;
mod errors;
mod lifecycle;

pub use a4n_domain::collaboration::{
    ProjectAccessRole, ProjectAccessScope, ProjectInviteSummary, ProjectMemberRole,
    ProjectMemberSummary,
};
pub use contracts::{
    AccessibleProjectListInput, AccessibleProjectRecord, AccessibleProjectsPage,
    CollaborationRootProject, ProjectAccessInfo, ProjectAccessInfoQuery,
};
pub(in crate::collaboration) use contracts::{ProjectAccessRow, UserAccessRow, UserDisplayRow};
pub use errors::CollaborationDbError;

pub(in crate::collaboration) use lifecycle::{
    INVITE_STATUS_ACCEPTED, INVITE_STATUS_DECLINED, INVITE_STATUS_EXPIRED, INVITE_STATUS_PENDING,
    INVITE_STATUS_REVOKED, INVITE_TTL_DAYS,
};
