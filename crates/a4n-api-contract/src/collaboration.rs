use schemars::JsonSchema;
use serde::Deserialize;

pub use a4n_domain::collaboration::{
    ProjectAccessRole, ProjectAccessScope, ProjectInviteSummary, ProjectMemberRole,
    ProjectMemberSummary,
};

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct InviteProjectMemberInput {
    pub username_or_email: String,
    pub role: ProjectMemberRole,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct UpdateProjectMemberRoleInput {
    pub role: ProjectMemberRole,
}
