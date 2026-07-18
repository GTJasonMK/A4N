//! 应用全局运行时状态。
//!
//! 这个状态不依赖 Tauri 或 Axum，用于让桌面端和 Web server 共享同一套
//! services、engine、队列和存储初始化逻辑。

mod app;
mod database;
mod profile;
mod prompt_engine;
mod rag;

pub use app::AppState;
pub use profile::RuntimeProfile;

#[cfg(test)]
mod convergence_tests;
#[cfg(test)]
mod tests;
