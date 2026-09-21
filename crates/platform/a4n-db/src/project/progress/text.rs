use super::{ProjectProgressQuery, ProjectProgressSummary};

pub(super) fn text_project_progress(project: &ProjectProgressQuery) -> ProjectProgressSummary {
    ProjectProgressSummary {
        label: project.status.to_string(),
        detail: (project.target_chapters > 0).then(|| {
            format!(
                "当前章节 {}/{}",
                project.current_chapter, project.target_chapters
            )
        }),
    }
}
