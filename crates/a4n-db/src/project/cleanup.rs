mod artifacts;
mod domains;
mod execution;
mod media_paths;
mod types;

pub(crate) use domains::{
    delete_project_blueprints, delete_project_chapter_adaptation_records,
    delete_project_conversations, delete_project_optimization_tree, delete_project_text_anchors,
};
pub use types::ProjectCleanupDbError;
pub(crate) use types::{
    ProjectBlueprintCleanupResult, ProjectChapterAdaptationDeleteScope,
    ProjectConversationCleanupResult, ProjectGeneratedArtifactLinkCleanupResult,
    ProjectGeneratedArtifactRowCleanupResult, ProjectGenerationMediaGraphCleanupResult,
    ProjectOptimizationCleanupResult, ProjectTaskQueueCleanupResult,
    ProjectTextAnchorCleanupResult, ProjectTextAnchorDeleteScope,
};

pub(crate) use artifacts::{
    delete_project_generated_artifact_links, delete_project_generated_artifact_rows,
    delete_project_generation_media_graph,
};
pub(crate) use execution::{
    delete_project_generation_runs, delete_project_task_queue_records, delete_project_workflow_runs,
};
pub(crate) use media_paths::project_generated_media_paths;

#[cfg(test)]
mod tests;
