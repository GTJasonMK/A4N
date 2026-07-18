use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectWorkflowCleanupResult {
    pub deleted_workflow_runs: u64,
    pub deleted_workflow_checkpoints: u64,
    pub deleted_workflow_progress_events: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectGenerationRunCleanupResult {
    pub deleted_generation_runs: u64,
    pub deleted_generation_run_targets: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectTaskQueueCleanupResult {
    pub deleted_tasks: u64,
    pub deleted_task_dependencies: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectGeneratedArtifactLinkCleanupResult {
    pub deleted_media_bindings: u64,
    pub deleted_media_provenance: u64,
    pub deleted_generation_spec_media_refs: u64,
    pub deleted_generation_spec_text_anchors: u64,
    pub deleted_generation_spec_entity_refs: u64,
    pub deleted_prompt_artifacts: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectGeneratedArtifactRowCleanupResult {
    pub deleted_generation_specs: u64,
    pub deleted_media_assets: u64,
    pub deleted_generated_images: u64,
    pub deleted_generated_videos: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectGenerationMediaGraphCleanupResult {
    pub deleted_generation_specs: u64,
    pub deleted_prompt_artifacts: u64,
    pub deleted_media_assets: u64,
    pub deleted_media_provenance: u64,
    pub deleted_media_bindings: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectOptimizationCleanupResult {
    pub deleted_sessions: u64,
    pub deleted_session_messages: u64,
    pub deleted_turns: u64,
    pub deleted_events: u64,
    pub deleted_patch_sets: u64,
    pub deleted_patches: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectConversationCleanupResult {
    pub deleted_conversations: u64,
    pub deleted_messages: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectBlueprintCleanupResult {
    pub deleted_current_blueprints: u64,
    pub deleted_blueprint_characters: u64,
    pub deleted_blueprint_relationships: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProjectTextAnchorDeleteScope {
    Project,
    CurrentProjectContent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectTextAnchorCleanupResult {
    pub deleted_text_anchors: u64,
    pub deleted_profile_evidence: u64,
}
