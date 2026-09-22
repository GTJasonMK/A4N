use a4n_domain::project::ProjectStatus;

use super::result::{GalgameResetOutcomeInput, galgame_reset_outcome};
use crate::galgame::GalgameScriptCleanupResult;
use crate::project::delete::ChapterGenerationCleanupResult;
use crate::project::{
    ProjectConversationCleanupResult, ProjectGenerationMediaGraphCleanupResult,
    ProjectOptimizationCleanupResult, ProjectTaskQueueCleanupResult,
};

fn input(previous_status: ProjectStatus) -> GalgameResetOutcomeInput {
    GalgameResetOutcomeInput {
        previous_status,
        optimization_cleanup: ProjectOptimizationCleanupResult {
            deleted_sessions: 11,
            deleted_session_messages: 12,
            deleted_turns: 13,
            deleted_events: 14,
            deleted_patch_sets: 15,
            deleted_patches: 16,
        },
        chapter_generation_cleanup: ChapterGenerationCleanupResult {
            deleted_generated_images: 21,
            deleted_generated_videos: 22,
            ..Default::default()
        },
        generation_media_cleanup: ProjectGenerationMediaGraphCleanupResult {
            deleted_generation_specs: 31,
            deleted_prompt_artifacts: 32,
            deleted_media_assets: 33,
            deleted_media_provenance: 34,
            deleted_media_bindings: 35,
        },
        galgame_cleanup: GalgameScriptCleanupResult {
            deleted_scripts: 41,
            deleted_scenes: 42,
            deleted_lines: 43,
            deleted_choices: 44,
        },
        conversation_cleanup: ProjectConversationCleanupResult {
            deleted_conversations: 51,
            deleted_messages: 52,
        },
        task_cleanup: ProjectTaskQueueCleanupResult {
            deleted_tasks: 61,
            deleted_task_dependencies: 62,
        },
        media_paths: vec!["media/a.png".to_string()],
    }
}

#[test]
fn galgame_reset_outcome_projects_cleanup_counts() {
    let outcome = galgame_reset_outcome(input(ProjectStatus::Writing));

    assert_eq!(outcome.result.deleted_galgame_scripts, 41);
    assert_eq!(outcome.result.deleted_galgame_scenes, 42);
    assert_eq!(outcome.result.deleted_galgame_lines, 43);
    assert_eq!(outcome.result.deleted_galgame_choices, 44);
    assert_eq!(outcome.result.deleted_generated_images, 21);
    assert_eq!(outcome.result.deleted_generated_videos, 22);
    assert_eq!(outcome.result.deleted_generation_specs, 31);
    assert_eq!(outcome.result.deleted_conversations, 51);
    assert_eq!(outcome.result.deleted_tasks, 61);
    assert_eq!(outcome.media_paths, vec!["media/a.png"]);
}

#[test]
fn galgame_reset_outcome_preserves_previous_draft_status() {
    let outcome = galgame_reset_outcome(input(ProjectStatus::Draft));

    assert_eq!(outcome.previous_status, ProjectStatus::Draft);
}
