use a4n_core::state::status::ProjectStatus;

use super::{super::ProjectStatusArtifacts, artifacts::next_status_from_artifact_flags};

#[test]
fn next_status_from_artifacts_keeps_blueprint_ready_without_blueprint() {
    let next = next_status_from_artifact_flags(
        ProjectStatus::BlueprintReady,
        ProjectStatusArtifacts {
            has_chapter_outline: true,
            ..Default::default()
        },
    );

    assert_eq!(next, None);
}

#[test]
fn next_status_from_artifacts_prefers_part_outlines_over_chapter_outlines() {
    let next = next_status_from_artifact_flags(
        ProjectStatus::BlueprintReady,
        ProjectStatusArtifacts {
            has_blueprint: true,
            has_part_outline: true,
            has_chapter_outline: true,
            ..Default::default()
        },
    );

    assert_eq!(next, Some(ProjectStatus::PartOutlinesReady));
}

#[test]
fn next_status_from_artifacts_advances_writing_from_chapter_content() {
    let next = next_status_from_artifact_flags(
        ProjectStatus::ChapterOutlinesReady,
        ProjectStatusArtifacts {
            has_chapter_content: true,
            ..Default::default()
        },
    );

    assert_eq!(next, Some(ProjectStatus::Writing));
}
