//! 角色隐性属性复审 action。
//!
//! Web 和 Tauri 共用这里的统计、模型请求、决策应用与 RAG 同步逻辑。

mod api;
mod application;
mod checkpoint;
mod context;
mod contracts;
mod decision;
mod projections;
mod prompt;
mod request;
mod stats;
mod types;

pub use api::{check_implicit_update, get_implicit_review, preview_check_implicit_update_request};
pub use application::apply_implicit_update_decision;
pub(crate) use checkpoint::{
    implicit_update_decision_output_json, implicit_update_review_checkpoint_response,
};
pub(crate) use contracts::ImplicitUpdateLlmRequestInput;
pub use contracts::{
    ApplyImplicitUpdateDecisionInput, ApplyImplicitUpdateDecisionResultSummary,
    AttributeEventSummary, CheckImplicitUpdateInput, DeletionMarkSummary, GetImplicitReviewInput,
};
pub(crate) use decision::parse_implicit_update_decision;
pub(crate) use prompt::IMPLICIT_UPDATE_PROMPT_NAME;
pub(crate) use request::build_implicit_update_request_for_state;
pub(crate) use stats::{
    build_implicit_attribute_review, build_implicit_review_summary, validate_implicit_review_params,
};
pub use types::{
    AUTO_IMPLICIT_REVIEW_EVIDENCE_LIMIT, AUTO_IMPLICIT_REVIEW_THRESHOLD,
    AUTO_IMPLICIT_REVIEW_WINDOW_CHAPTERS, BehaviorForReview, ImplicitAttributeReviewSummary,
    ImplicitReviewSummary, ImplicitUpdateDecisionSummary,
};

#[cfg(test)]
mod tests;
