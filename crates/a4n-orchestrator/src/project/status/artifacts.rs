use a4n_domain::project::ProjectStatus;

use super::super::ProjectStatusArtifacts;

pub(crate) fn next_status_from_artifact_flags(
    status: ProjectStatus,
    artifacts: ProjectStatusArtifacts,
) -> Option<ProjectStatus> {
    let actual = if artifacts.has_chapter_content {
        ProjectStatus::Writing
    } else if artifacts.has_chapter_outline {
        ProjectStatus::ChapterOutlinesReady
    } else if artifacts.has_part_outline {
        ProjectStatus::PartOutlinesReady
    } else if artifacts.has_blueprint {
        ProjectStatus::BlueprintReady
    } else {
        ProjectStatus::Draft
    };
    (actual.ordinal() > status.ordinal()).then_some(actual)
}
