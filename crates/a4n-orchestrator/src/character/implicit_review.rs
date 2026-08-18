//! 角色隐性属性复审 application。
//!
//! 统一拥有统计、请求构造、workflow/checkpoint、LLM 调用、严格输出校验和决策写入。

mod contracts;
mod execution;
mod output;
mod request;
mod stats;

pub use contracts::{
    ApplyImplicitUpdateDecisionInput, CheckImplicitUpdateInput, GetImplicitReviewInput,
    ImplicitAttributeReviewSummary, ImplicitEvidenceSummary, ImplicitReviewSummary,
    ImplicitUpdateDecisionSummary,
};
pub(crate) use contracts::CharacterSyncImplicitReviewInput;

pub const IMPLICIT_UPDATE_PROMPT_NAME: &str = "implicit_update";
pub const IMPLICIT_UPDATE_WORKFLOW_KIND: &str = "implicit_update_review";
const IMPLICIT_UPDATE_STEP_KEY: &str = "implicit_update_decision";

#[cfg(test)]
mod tests;
