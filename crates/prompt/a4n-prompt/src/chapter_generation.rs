//! 章节生成 LLM 请求合同。

mod manifest;
mod message;
mod request;

pub use request::build_chapter_generation_request;
