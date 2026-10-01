//! Database boundary types for orchestrator-facing code.
//!
//! Repository implementations can move here incrementally. This crate should
//! own database connection and transaction abstractions instead of letting host
//! entry crates reach into SeaORM directly.

pub mod admin;
pub mod agent;
pub mod asset;
pub mod auth;
pub mod blueprint;
pub mod canvas;
pub mod chapter_content;
pub mod chapter_visual_plan;
pub mod character;
pub mod cleanup_outbox;
pub mod collaboration;
pub mod config;
pub mod export;
pub mod fact_feedback;
pub mod galgame;
pub mod generation;
pub mod import_reverse;
pub mod import_review;
pub mod inspiration;
pub mod llm_context;
pub mod manga;
pub mod media;
pub mod media_authoring;
pub mod model_usage;
pub mod optimize;
pub mod outline;
pub mod package;
pub mod profile_material;
pub mod project;
pub mod prompt_override;
pub mod queue;
pub mod rag;
pub mod scene_profile;
pub mod shooting_script;
pub mod source_sync;
pub mod text_anchor;
pub mod video;
pub mod workflow;

pub type DatabaseConnection = sea_orm::DatabaseConnection;
pub use a4n_db_session::{DbHandle, DbTransaction, SharedDatabaseConnection};
