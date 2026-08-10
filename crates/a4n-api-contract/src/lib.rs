//! 唯一的 Web/Tauri Host API 合同源。
//!
//! 这里只放跨进程 Host 请求/响应的 serde 合同；数据库实体、provider、runtime
//! 状态和 application 规则不得进入本 crate。

pub mod admin;
pub mod auth;
pub mod blueprint;
pub mod canvas;
pub mod chapter;
pub mod character;
pub mod collaboration;
pub mod command_manifest;
pub mod config;
pub mod embedding;
pub mod export;
pub mod galgame;
pub mod generation_tasks;
pub mod image;
pub mod image_generation;
pub mod import;
pub mod import_reverse;
pub mod import_review;
pub mod inspiration;
pub mod logging;
pub mod manga;
pub mod optimize;
pub mod outline;
pub mod portrait;
pub mod profile;
pub mod project;
pub mod project_transfer;
pub mod prompt;
pub mod provider_catalog;
pub mod rag;
pub mod request_preview;
mod serde_update;
pub mod shooting_script;
pub mod source_sync;
pub mod storage;
pub mod task;
pub mod temp_media;
pub mod theme;
pub mod video;
pub mod workflow;
