//! 角色隐性属性复审 application。
//!
//! 统一拥有统计、请求构造、workflow/checkpoint、LLM 调用、严格输出校验和决策写入。

mod contracts;
mod execution;
mod output;
mod request;
mod stats;

pub use contracts::{
    ApplyImplicitUpdateDecisionInput, ApplyImplicitUpdateDecisionResultSummary,
    AttributeEventSummary, CharacterSyncImplicitReviewInput, CheckImplicitUpdateInput,
    DeletionMarkSummary, GetImplicitReviewInput, ImplicitAttributeReviewSummary,
    ImplicitEvidenceSummary, ImplicitReviewSummary, ImplicitUpdateDecisionSummary,
    ImplicitUpdateReviewRequestPreview,
};

pub const IMPLICIT_UPDATE_PROMPT_NAME: &str = "implicit_update";

#[cfg(test)]
mod tests;
