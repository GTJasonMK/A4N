/// 业务实现体（crate 内）。宿主请使用 [`host_api`]。
pub(crate) mod actions;
/// 阶段 11：宿主侧唯一业务编排入口（orchestrator facade）。
///
/// `a4n-server` / `a4n-tauri` 只能通过本模块调用业务。
/// 实现体在 `actions`（`pub(crate)`），对外不暴露。
pub mod host_api;

mod baseline_configs;
mod cleanup_outbox;
pub(crate) mod dynamic_embedding;
pub(crate) mod dynamic_llm;
pub(crate) mod file_utils;
pub(crate) mod jwt;
pub(crate) mod oauth_provider;
mod prompt_resources;
pub(crate) mod provider_error;
pub(crate) mod provider_runtime;
pub(crate) mod rag_ingest;
pub(crate) mod runtime_config;
mod service_health;
pub(crate) mod sidecar_clients;
pub(crate) mod state;
pub(crate) mod workflow;

pub use state::{AppState, RuntimeProfile};

#[cfg(test)]
mod media_lifecycle_tests;
