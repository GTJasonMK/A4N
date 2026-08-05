//! A4N 核心库：数据模型、共享类型、状态机、事件系统
//!
//! 本 crate 是所有其他 crate 的基础依赖，不依赖任何内部 crate。

pub mod blueprint;
pub mod blueprint_context;
pub mod chapter_analysis;
pub mod config;
pub mod dependency_conflict;
pub mod error;
pub mod event;
pub mod files;
pub mod import_reverse;
pub mod json_contract;
pub mod manga;
pub mod models;
pub mod profile_text;
pub mod prompt_language;
pub mod rag;
pub mod schema;
pub mod shooting_script;
pub mod source_spans;
pub mod state;
pub mod structured_progress;
pub mod text_location;
pub mod video_execution;
pub mod video_workflow;
