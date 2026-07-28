use a4n_core::state::status::ProjectStatus;

use super::super::ProjectStatusArtifacts;

pub(crate) fn next_status_from_artifact_flags(
    status: ProjectStatus,
    artifacts: ProjectStatusArtifacts,
) -> Option<ProjectStatus> {
    match status {
        ProjectStatus::Draft => artifacts
            .has_blueprint
            .then_some(ProjectStatus::BlueprintReady),
        ProjectStatus::BlueprintReady => {
            if !artifacts.has_blueprint {
                return None;
            }
            if artifacts.has_part_outline {
                return Some(ProjectStatus::PartOutlinesReady);
            }
            artifacts
                .has_chapter_outline
                .then_some(ProjectStatus::ChapterOutlinesReady)
        }
        ProjectStatus::PartOutlinesReady => artifacts
            .has_chapter_outline
            .then_some(ProjectStatus::ChapterOutlinesReady),
        ProjectStatus::ChapterOutlinesReady => artifacts
            .has_chapter_content
            .then_some(ProjectStatus::Writing),
        ProjectStatus::Writing | ProjectStatus::Completed => None,
    }
}
