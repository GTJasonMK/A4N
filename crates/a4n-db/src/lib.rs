//! Database boundary types for orchestrator-facing code.
//!
//! Repository implementations can move here incrementally. This crate should
//! own database connection and transaction abstractions instead of letting host
//! entry crates reach into SeaORM directly.

pub mod admin;
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
pub mod galgame;
pub mod generation;
pub mod import_reverse;
pub mod import_review;
pub mod inspiration;
pub mod llm_context;
pub mod manga;
pub mod media;
pub mod media_workflow_reset;
pub mod optimize;
pub mod outline;
pub mod profile_material;
pub mod project;
pub mod project_delete;
pub mod project_dependency_graph;
pub mod project_family_import;
pub mod project_reset;
pub mod prompt_override;
pub mod queue;
pub mod rag;
pub mod scene_profile;
pub mod shooting_script;
pub mod source_sync;
pub mod text_anchor;
pub mod video;
pub mod video_prompt;
pub mod workflow;

use std::sync::Arc;

pub type DatabaseConnection = sea_orm::DatabaseConnection;
pub type SharedDatabaseConnection = Arc<DatabaseConnection>;

#[derive(Clone)]
pub struct DbHandle {
    connection: SharedDatabaseConnection,
}

impl DbHandle {
    pub fn new(connection: SharedDatabaseConnection) -> Self {
        Self { connection }
    }

    pub fn connection(&self) -> &DatabaseConnection {
        self.connection.as_ref()
    }

    pub fn shared_connection(&self) -> SharedDatabaseConnection {
        Arc::clone(&self.connection)
    }
}
