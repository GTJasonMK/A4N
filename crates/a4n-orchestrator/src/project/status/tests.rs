use a4n_domain::project::ProjectStatus;

use super::{
    super::ProjectStatusArtifacts, artifacts::next_status_from_artifact_flags, rules::rollback_path,
};

#[test]
fn next_status_from_artifacts_uses_chapter_outline_without_blueprint() {
    let next = next_status_from_artifact_flags(
        ProjectStatus::BlueprintReady,
        ProjectStatusArtifacts {
            has_chapter_outline: true,
            ..Default::default()
        },
    );

    assert_eq!(next, Some(ProjectStatus::ChapterOutlinesReady));
}

#[test]
fn next_status_from_artifacts_prefers_furthest_existing_artifact() {
    let next = next_status_from_artifact_flags(
        ProjectStatus::BlueprintReady,
        ProjectStatusArtifacts {
            has_blueprint: true,
            has_part_outline: true,
            has_chapter_outline: true,
            ..Default::default()
        },
    );

    assert_eq!(next, Some(ProjectStatus::ChapterOutlinesReady));
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

#[test]
fn next_status_from_artifacts_enters_writing_without_blueprint_or_outline() {
    let next = next_status_from_artifact_flags(
        ProjectStatus::Draft,
        ProjectStatusArtifacts {
            has_chapter_content: true,
            ..Default::default()
        },
    );

    assert_eq!(next, Some(ProjectStatus::Writing));
}

#[test]
fn rollback_path_includes_typed_endpoints() {
    assert_eq!(
        rollback_path(ProjectStatus::Writing, ProjectStatus::BlueprintReady),
        vec![
            ProjectStatus::Writing,
            ProjectStatus::ChapterOutlinesReady,
            ProjectStatus::PartOutlinesReady,
            ProjectStatus::BlueprintReady,
        ]
    );
}
