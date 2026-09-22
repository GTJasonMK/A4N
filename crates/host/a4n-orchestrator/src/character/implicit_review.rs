//! 角色隐性属性复审 application。
//!
//! 统一拥有统计、请求构造、workflow/checkpoint、LLM 调用、严格输出校验和决策写入。

pub(super) mod rules;
pub(super) mod workflow;

mod contracts;
mod execution;
mod output;
mod request;
mod stats;

pub(crate) use contracts::CharacterSyncImplicitReviewInput;

pub(crate) use a4n_character_host_contract::{
    IMPLICIT_UPDATE_PROMPT_NAME, IMPLICIT_UPDATE_WORKFLOW_KIND,
};
const IMPLICIT_UPDATE_STEP_KEY: &str = "implicit_update_decision";

#[cfg(test)]
mod tests;
