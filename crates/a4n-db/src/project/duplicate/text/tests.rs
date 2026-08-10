use super::rules::{
    project_copy_status_includes_blueprint, project_copy_status_includes_chapter_outlines,
    project_copy_status_includes_chapters, project_copy_status_includes_part_outlines,
};
use a4n_domain::project::ProjectStatus;

#[test]
fn draft_branch_excludes_all_generated_text_graphs() {
    assert!(!project_copy_status_includes_blueprint(
        ProjectStatus::Draft
    ));
    assert!(!project_copy_status_includes_part_outlines(
        ProjectStatus::Draft
    ));
    assert!(!project_copy_status_includes_chapter_outlines(
        ProjectStatus::Draft
    ));
    assert!(!project_copy_status_includes_chapters(ProjectStatus::Draft));
}

#[test]
fn branch_status_thresholds_enable_text_graphs_incrementally() {
    assert!(project_copy_status_includes_blueprint(
        ProjectStatus::BlueprintReady
    ));
    assert!(!project_copy_status_includes_part_outlines(
        ProjectStatus::BlueprintReady
    ));

    assert!(project_copy_status_includes_part_outlines(
        ProjectStatus::PartOutlinesReady
    ));
    assert!(!project_copy_status_includes_chapter_outlines(
        ProjectStatus::PartOutlinesReady
    ));

    assert!(project_copy_status_includes_chapter_outlines(
        ProjectStatus::ChapterOutlinesReady
    ));
    assert!(!project_copy_status_includes_chapters(
        ProjectStatus::ChapterOutlinesReady
    ));

    assert!(project_copy_status_includes_chapters(
        ProjectStatus::Writing
    ));
    assert!(project_copy_status_includes_chapters(
        ProjectStatus::Completed
    ));
}
