//! Business orchestration boundary.
//!
//! Runtime actions can move here incrementally. Host crates should eventually
//! call this crate instead of owning business transactions directly.

pub mod admin;
pub mod asset;
pub use a4n_db::asset_lineage_sync;
pub mod auth;
pub mod blueprint;
pub mod canvas;
pub mod chapter_analysis;
pub mod chapter_content;
pub mod chapter_generation;
pub mod chapter_scene;
pub mod character;
pub mod collaboration;
pub mod config;
pub mod export;
pub mod galgame;
pub mod generation;
pub mod import_analysis;
pub mod import_reverse;
pub mod import_review;
pub mod inspiration;
pub use a4n_db::llm_context;
pub mod manga;
pub mod media;
pub mod object_discovery;
pub mod optimize;
pub mod outline;
pub mod portrait;
pub mod profile_material;
pub mod project;
pub mod prompt_records;
pub mod rag;
pub mod scene_profile;
pub mod shooting_script;
pub mod source_sync;
pub mod structured_llm;
pub mod task_queue;
pub mod text_anchor;
pub mod theme;
pub mod video;
pub mod workflow;

mod context;
mod discovery_workflow;
mod errors;
mod service_health;

#[cfg(test)]
pub(crate) mod test_support;

pub use context::dynamic_provider::DynamicProviderFacade;
pub use context::{
    GenerationTaskAccessError, GenerationTaskHistorySummaries, GenerationTaskStart,
    OrchestratorContext,
};
pub use service_health::ManagedServiceHealthSummary;

pub(crate) use errors::{
    chapter_content_edit_error_to_app_error, freeform_generated_reset_error_to_app_error,
    project_delete_error_to_app_error,
};

#[cfg(test)]
mod tests;
