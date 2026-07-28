//! 应用全局运行时状态。
//!
//! 这个状态不依赖 Tauri 或 Axum，用于让桌面端和 Web server 共享同一套
//! services、engine、队列和存储初始化逻辑。

mod app;
mod database;
mod profile;
mod prompt_engine;
mod rag;

mod admin;
mod asset;
mod asset_lineage_sync;
mod auth;
mod canvas;
mod chapter_analysis;
mod chapter_content;
mod chapter_generation;
mod character;
mod collaboration;
mod export;
mod galgame;
mod generation_tasks;
mod import_analysis;
mod import_reverse;
mod import_review;
mod inspiration;
mod manga;
mod media;
mod optimize;
mod outline;
mod portrait;
mod profile_material;
mod project_blueprint;
mod project_chapter_records;
mod project_dependency_graph;
mod project_duplication;
mod project_family;
mod project_mutation;
mod project_outline_records;
mod project_read;
mod project_scene_records;
mod project_status;
mod prompt_records;
mod scene_profile;
mod settings;
mod shooting_script;
mod source_sync;
mod task_queue;
mod video;
mod video_studio_transfer;
mod workflow;

pub use app::AppState;
pub use profile::RuntimeProfile;

// Domain facades are crate-internal contracts used by action helpers. Keeping
// these types visible only inside runtime lets helpers depend on the smallest
// domain surface without exposing the full AppState to every call chain.
pub(crate) use asset::AssetRuntimeFacade;
pub(crate) use canvas::CanvasRuntimeFacade;
pub(crate) use collaboration::CollaborationRuntimeFacade;
pub(crate) use profile_material::ProfileMaterialRuntimeFacade;
pub(crate) use project_outline_records::ProjectOutlineRecordsRuntimeFacade;
pub(crate) use project_read::ProjectReadRuntimeFacade;
pub(crate) use rag::RagRuntimeFacade;
pub(crate) use scene_profile::SceneProfileRuntimeFacade;
pub(crate) use settings::SettingsRuntimeFacade;
pub(crate) use video::VideoRuntimeFacade;

#[cfg(test)]
mod convergence_tests;
#[cfg(test)]
mod tests;
