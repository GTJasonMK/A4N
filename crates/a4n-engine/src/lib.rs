//! A4N 引擎层：LLM调用、RAG检索、嵌入计算、提示词渲染、任务队列
//!
//! 依赖 a4n-core，不依赖 a4n-services。

pub mod embedding;
pub mod image;
pub mod llm;
pub mod prompt;
pub mod queue;
pub mod rag;
pub mod video;
