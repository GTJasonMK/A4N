//! 角色隐性属性复审 Host 适配。

mod api;
mod application;

pub use a4n_orchestrator::character::{
    AUTO_IMPLICIT_REVIEW_EVIDENCE_LIMIT, AUTO_IMPLICIT_REVIEW_THRESHOLD,
    AUTO_IMPLICIT_REVIEW_WINDOW_CHAPTERS, ApplyImplicitUpdateDecisionInput,
    ApplyImplicitUpdateDecisionResultSummary, AttributeEventSummary, CheckImplicitUpdateInput,
    DeletionMarkSummary, GetImplicitReviewInput, IMPLICIT_UPDATE_PROMPT_NAME,
    ImplicitAttributeReviewSummary, ImplicitEvidenceSummary, ImplicitReviewSummary,
    ImplicitUpdateDecisionSummary,
};
pub use api::{check_implicit_update, get_implicit_review, preview_check_implicit_update_request};
pub use application::apply_implicit_update_decision;
