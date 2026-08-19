mod artifacts;
mod domains;
mod execution;
mod media_paths;
mod types;

pub use domains::{
    delete_project_blueprints, delete_project_conversations, delete_project_optimization_tree,
    delete_project_text_anchors,
};
pub use types::{
    ProjectBlueprintCleanupResult, ProjectCleanupDbError, ProjectConversationCleanupResult,
    ProjectGeneratedArtifactLinkCleanupResult, ProjectGeneratedArtifactRowCleanupResult,
    ProjectGenerationMediaGraphCleanupResult, ProjectGenerationRunCleanupResult,
    ProjectOptimizationCleanupResult, ProjectTaskQueueCleanupResult,
    ProjectTextAnchorCleanupResult, ProjectTextAnchorDeleteScope, ProjectWorkflowCleanupResult,
};

pub(crate) use artifacts::*;
pub(crate) use execution::*;
pub(crate) use media_paths::*;

#[cfg(test)]
mod tests;
