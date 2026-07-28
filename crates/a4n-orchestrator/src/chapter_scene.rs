//! 章节场景生成 application。
//!
//! 统一拥有输入合同、上下文与请求构造、workflow/checkpoint、LLM 调用、输出校验和落库。

mod context;
mod contracts;
mod execution;
mod output;
mod request;
mod workflow;

pub use contracts::{
    ChapterSceneGenerationApplicationInput, ChapterSceneGenerationInput,
    ChapterSceneProgressReporter, ChapterSceneRequestPreview, ChapterSceneTaskScope,
};

#[cfg(test)]
mod tests;
