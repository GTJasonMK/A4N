/// 业务实现体（crate 内）。宿主请使用 [`host_api`]。
pub(crate) mod actions;
/// 阶段 11：宿主侧唯一业务编排入口（orchestrator facade）。
///
/// `a4n-server` / `a4n-tauri` 只能通过本模块调用业务。
/// 实现体在 `actions`（`pub(crate)`），对外不暴露。
pub mod host_api;

mod baseline_configs;
mod cleanup_outbox;
pub mod dynamic_embedding;
pub mod dynamic_llm;
pub mod file_utils;
pub(crate) mod jwt;
pub mod media_file;
pub(crate) mod oauth_provider;
pub mod rag_ingest;
pub mod runtime_config;
pub(crate) mod sidecar_clients;
pub mod state;
pub mod workflow;

pub use state::AppState;
