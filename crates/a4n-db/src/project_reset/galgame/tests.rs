use a4n_core::models::project::novel_project;
use a4n_domain::project::ProjectStatus;
use chrono::NaiveDateTime;

use super::result::{GalgameResetOutcomeInput, galgame_reset_outcome};
use crate::galgame::GalgameScriptCleanupResult;
use crate::project::{
    ProjectConversationCleanupResult, ProjectGenerationMediaGraphCleanupResult,
    ProjectOptimizationCleanupResult, ProjectTaskQueueCleanupResult,
};
use crate::project_delete::ChapterGenerationCleanupResult;

fn project() -> novel_project::Model {
    novel_project::Model {
        id: 7,
        user_id: 1,
        title: "Galgame".to_string(),
        description: String::new(),
        project_type: "script".to_string(),
        project_kind: "galgame".to_string(),
        root_project_id: Some(3),
        source_snapshot_id: Some(5),
        source_sync_status: "current".to_string(),
        status: "draft".to_string(),
        lifecycle_gate: "active".to_string(),
        target_chapters: 0,
        current_chapter: 0,
        total_chapters: None,
        chapters_per_part: None,
        avatar_svg: None,
        avatar_animal: None,
        raw_blueprint: None,
        import_analysis_progress: None,
        export_config: None,
        created_at: NaiveDateTime::MIN,
        updated_at: NaiveDateTime::MIN,
    }
}

fn input(previous_status: ProjectStatus) -> GalgameResetOutcomeInput {
    GalgameResetOutcomeInput {
        project: project(),
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
