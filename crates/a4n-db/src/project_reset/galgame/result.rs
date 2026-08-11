use a4n_core::models::project::novel_project;
use a4n_domain::project::ProjectStatus;

use super::super::types::{ProjectGeneratedDataResetOutcome, ProjectGeneratedDataResetResult};
use crate::galgame::GalgameScriptCleanupResult;
use crate::project::{
    ProjectConversationCleanupResult, ProjectGenerationMediaGraphCleanupResult,
    ProjectOptimizationCleanupResult, ProjectTaskQueueCleanupResult,
};
use crate::project_delete::ChapterGenerationCleanupResult;

pub(super) struct GalgameResetOutcomeInput {
    pub project: novel_project::Model,
    pub previous_status: ProjectStatus,
    pub optimization_cleanup: ProjectOptimizationCleanupResult,
    pub chapter_generation_cleanup: ChapterGenerationCleanupResult,
    pub generation_media_cleanup: ProjectGenerationMediaGraphCleanupResult,
    pub galgame_cleanup: GalgameScriptCleanupResult,
    pub conversation_cleanup: ProjectConversationCleanupResult,
    pub task_cleanup: ProjectTaskQueueCleanupResult,
    pub media_paths: Vec<String>,
}

pub(super) fn galgame_reset_outcome(
    input: GalgameResetOutcomeInput,
) -> ProjectGeneratedDataResetOutcome {
    ProjectGeneratedDataResetOutcome {
        result: ProjectGeneratedDataResetResult {
            project: input.project,
            deleted_galgame_scripts: input.galgame_cleanup.deleted_scripts,
            deleted_galgame_scenes: input.galgame_cleanup.deleted_scenes,
            deleted_galgame_lines: input.galgame_cleanup.deleted_lines,
            deleted_galgame_choices: input.galgame_cleanup.deleted_choices,
            deleted_chapters: 0,
            deleted_chapter_outlines: 0,
            deleted_part_outlines: 0,
            deleted_chapter_derived_rows: 0,
            deleted_current_blueprints: 0,
            deleted_blueprint_characters: 0,
            deleted_blueprint_relationships: 0,
            preserved_blueprint_versions: 0,
            reset_character_profiles: 0,
            deleted_character_profiles: 0,
            deleted_character_snapshots: 0,
            deleted_character_timeline_rows: 0,
            deleted_character_portraits: 0,
            deleted_object_profiles: 0,
            deleted_foreshadowing: 0,
            deleted_scene_profiles: 0,
            deleted_scene_occurrences: 0,
            deleted_scene_transitions: 0,
            deleted_scene_entity_states: 0,
            deleted_text_anchors: 0,
            deleted_profile_evidence: 0,
            deleted_import_reviews: 0,
            deleted_conversations: input.conversation_cleanup.deleted_conversations,
            deleted_conversation_messages: input.conversation_cleanup.deleted_messages,
            deleted_optimization_sessions: input.optimization_cleanup.deleted_sessions,
            deleted_optimization_session_messages: input
                .optimization_cleanup
                .deleted_session_messages,
            deleted_optimization_turns: input.optimization_cleanup.deleted_turns,
            deleted_optimization_events: input.optimization_cleanup.deleted_events,
            deleted_optimization_patch_sets: input.optimization_cleanup.deleted_patch_sets,
            deleted_optimization_patches: input.optimization_cleanup.deleted_patches,
            deleted_generated_images: input.chapter_generation_cleanup.deleted_generated_images,
            deleted_generated_videos: input.chapter_generation_cleanup.deleted_generated_videos,
            deleted_generation_specs: input.generation_media_cleanup.deleted_generation_specs,
            deleted_prompt_artifacts: input.generation_media_cleanup.deleted_prompt_artifacts,
            deleted_media_assets: input.generation_media_cleanup.deleted_media_assets,
            deleted_media_provenance: input.generation_media_cleanup.deleted_media_provenance,
            deleted_media_bindings: input.generation_media_cleanup.deleted_media_bindings,
            deleted_tasks: input.task_cleanup.deleted_tasks,
            deleted_task_dependencies: input.task_cleanup.deleted_task_dependencies,
        },
        previous_status: input.previous_status,
        media_paths: input.media_paths,
    }
}
