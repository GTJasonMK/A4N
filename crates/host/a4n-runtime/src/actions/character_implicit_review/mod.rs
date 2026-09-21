//! 角色隐性属性复审 Host 适配。

mod api;
mod application;

pub use a4n_api_contract::character::{
    ApplyImplicitUpdateDecisionInput, ApplyImplicitUpdateDecisionResultSummary,
    CheckImplicitUpdateInput, GetImplicitReviewInput, ImplicitReviewSummary,
    ImplicitUpdateDecisionSummary,
};
pub use a4n_character_host_contract::{IMPLICIT_UPDATE_PROMPT_NAME, IMPLICIT_UPDATE_WORKFLOW_KIND};
pub use api::{check_implicit_update, get_implicit_review, preview_check_implicit_update_request};
pub use application::apply_implicit_update_decision;
