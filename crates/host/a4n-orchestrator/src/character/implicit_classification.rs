//! 角色行为隐性属性分类 application。
//!
//! 统一拥有 prompt/stable context、workflow/checkpoint、LLM 调用、严格输出校验和状态投影。

mod application;
mod contracts;
mod execution;
mod output;
mod request;
#[cfg(test)]
mod tests;

pub(crate) use contracts::CharacterImplicitClassificationInput;

const IMPLICIT_CLASSIFICATION_KIND: &str = "implicit_classification";
const IMPLICIT_CLASSIFICATION_STEP_KEY: &str = "behavior_classification";
const IMPLICIT_CLASSIFICATION_CONTEXT_LEDGER_VERSION: &str = "implicit_classification_context_v1";
