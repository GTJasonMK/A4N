//! 角色隐性属性复审 Host 适配。

mod api;
mod application;

pub use a4n_api_contract::character::ApplyImplicitUpdateDecisionResultSummary;
pub use a4n_orchestrator::character::{
    ApplyImplicitUpdateDecisionInput, CheckImplicitUpdateInput, GetImplicitReviewInput,
    IMPLICIT_UPDATE_PROMPT_NAME, IMPLICIT_UPDATE_WORKFLOW_KIND, ImplicitReviewSummary,
    ImplicitUpdateDecisionSummary,
};
pub use api::{check_implicit_update, get_implicit_review, preview_check_implicit_update_request};
pub use application::apply_implicit_update_decision;
