mod contracts;
mod errors;
mod lifecycle;
mod roles;
mod scope;

pub use contracts::*;
pub use errors::CollaborationDbError;
pub use roles::{ProjectAccessRole, ProjectMemberRole, parse_project_member_role};
pub use scope::ProjectAccessScope;

pub(in crate::collaboration) use lifecycle::*;
pub(in crate::collaboration) use roles::{
    ACCESS_ROLE_OWNER, MEMBER_ROLE_EDITOR, MEMBER_ROLE_MANAGER, MEMBER_ROLE_VIEWER,
};
pub(in crate::collaboration) use scope::{
    PROJECT_KIND_COMIC, PROJECT_KIND_GALGAME, PROJECT_KIND_TEXT, PROJECT_KIND_VIDEO,
    SOURCE_MODE_FREEFORM, SOURCE_MODE_SELF,
};
