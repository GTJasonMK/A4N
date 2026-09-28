//! Business orchestration boundary.
//!
//! Runtime actions can move here incrementally. Host crates should eventually
//! call this crate instead of owning business transactions directly.

pub(crate) mod admin;
mod asset;
mod blueprint;
pub(crate) mod canvas;
pub mod chapter_analysis;
pub(crate) mod chapter_content;
pub(crate) mod chapter_generation;
pub(crate) mod chapter_scene;
pub(crate) mod chapter_visual_plan;
pub(crate) mod character;
pub(crate) mod export {
    //! 导出 application：拥有项目与 Galgame 导出、格式渲染、进度与取消语义。

    mod application;
}
pub(crate) mod fact_feedback;
pub(crate) mod galgame;
pub(crate) mod generation;
pub(crate) mod import_analysis;
pub(crate) mod import_reverse;
pub(crate) mod inspiration;
pub(crate) mod manga;
pub(crate) mod media;
pub(crate) mod media_authoring;
mod object_discovery;
pub(crate) mod optimize;
pub(crate) mod outline;
pub(crate) mod portrait;
mod profile_material;
pub(crate) mod project;
pub(crate) mod prompt_records;
pub mod rag;
pub(crate) mod scene_profile;
pub(crate) mod shooting_script;
pub(crate) mod source_sync;
mod structured_llm;
mod text_anchor;
pub(crate) mod video;
pub(crate) mod workflow;

mod context;
mod discovery_workflow;
mod errors;

#[cfg(test)]
pub(crate) mod test_support;

pub use context::OrchestratorContext;
pub(crate) use context::generation::GenerationTaskAccessError;
pub(crate) use context::generation::GenerationTaskStart;

pub(crate) use errors::{
    chapter_content_edit_error_to_app_error, freeform_generated_reset_error_to_app_error,
    project_delete_error_to_app_error,
};

#[cfg(test)]
mod tests;
