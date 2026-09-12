//! Business orchestration boundary.
//!
//! Runtime actions can move here incrementally. Host crates should eventually
//! call this crate instead of owning business transactions directly.

pub mod admin;
pub mod agent;
mod asset;
pub mod auth;
mod blueprint;
pub mod canvas;
pub mod chapter_analysis;
pub mod chapter_content;
pub mod chapter_generation;
pub mod chapter_scene;
pub mod chapter_visual_plan;
pub mod character;
pub mod cleanup_outbox;
pub(crate) mod collaboration;
pub mod config;
pub mod export;
pub mod galgame;
pub mod generation;
pub mod import_analysis;
pub mod import_reverse;
pub mod import_review;
pub mod inspiration;
pub mod manga;
pub mod media;
mod object_discovery;
pub mod optimize;
pub mod outline;
pub mod portrait;
mod profile_material;
pub mod project;
pub mod prompt_records;
pub mod rag;
pub mod scene_profile;
pub mod shooting_script;
pub mod source_sync;
mod structured_llm;
pub mod task_queue;
mod text_anchor;
mod theme;
pub mod video;
pub mod workflow;

mod context;
mod discovery_workflow;
mod errors;

#[cfg(test)]
pub(crate) mod test_support;

pub use blueprint::BlueprintGenerationApplicationPort;
pub use blueprint::BlueprintRequestPreviewApplicationPort;
pub use cleanup_outbox::CleanupOutboxApplicationPort;
pub use context::OrchestratorContext;
pub(crate) use context::generation::GenerationTaskStart;
pub use context::generation::{
    AuthorizedGenerationTask, GenerationApplicationPort, GenerationTaskAccessError,
    GenerationTaskHistorySummaries,
};
pub use outline::OutlinePreviewApplicationPort;
pub use profile_material::ProfileMaterialUploadApplicationPort;
pub use shooting_script::ShootingScriptGenerationApplicationPort;

pub(crate) use errors::{
    chapter_content_edit_error_to_app_error, freeform_generated_reset_error_to_app_error,
    project_delete_error_to_app_error,
};

#[cfg(test)]
mod tests;
