use super::shared::progress_count_detail;
use super::{ProjectProgressQuery, ProjectProgressStatus, ProjectProgressSummary};
use crate::project::PROJECT_KIND_TEXT;

pub(super) fn unknown_project_progress(project_kind: &str) -> ProjectProgressSummary {
    ProjectProgressSummary {
        kind: project_kind.to_string(),
        stage: "unknown".to_string(),
        label: "未知状态".to_string(),
        current: None,
        total: None,
        status: ProjectProgressStatus::Blocked,
        detail: Some(format!("未知项目类型：{project_kind}")),
    }
}

pub(super) fn text_project_progress(project: &ProjectProgressQuery) -> ProjectProgressSummary {
    let label = match project.status.as_str() {
        "draft" => "草稿",
        "blueprint_ready" => "蓝图就绪",
        "part_outlines_ready" => "分部大纲就绪",
        "chapter_outlines_ready" => "章节大纲就绪",
        "writing" => "写作中",
        "completed" => "已完成",
        _ => "未知状态",
    };
    let current = Some(i64::from(project.current_chapter.max(0)));
    let total = Some(i64::from(project.target_chapters.max(0)));
    ProjectProgressSummary {
        kind: PROJECT_KIND_TEXT.to_string(),
        stage: project.status.clone(),
        label: label.to_string(),
        current,
        total,
        status: if project.status == "completed" {
            ProjectProgressStatus::Completed
        } else if label == "未知状态" {
            ProjectProgressStatus::Blocked
        } else {
            ProjectProgressStatus::Idle
        },
        detail: progress_count_detail("当前章节", current, total),
    }
}
