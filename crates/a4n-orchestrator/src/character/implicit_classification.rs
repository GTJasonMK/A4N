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

pub use contracts::{CharacterImplicitClassificationInput, CharacterImplicitClassificationOutput};

pub const IMPLICIT_CLASSIFICATION_PROMPT_NAME: &str = "implicit_classification";
