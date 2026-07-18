use super::shared::{
    PipelineProjectState, ProgressCounts, non_zero, progress_count_detail, project_progress,
};
use super::{ProjectProgressReadError, ProjectProgressStatus, ProjectProgressSummary};
use crate::project::PROJECT_KIND_COMIC;
use a4n_core::models::chapter::chapter_manga_prompt;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};
use serde_json::Value;
use std::collections::HashMap;

const MANGA_PAGE_PROMPT_TYPE: &str = "page_generation";
const PIPELINE_STATUS_PROMPT_TYPE: &str = "pipeline_status";

pub(super) async fn comic_project_progress_by_project(
    db: &DatabaseConnection,
    project_ids: &[i64],
) -> Result<HashMap<i64, ProjectProgressSummary>, ProjectProgressReadError> {
    if project_ids.is_empty() {
        return Ok(HashMap::new());
    }
    let rows = chapter_manga_prompt::Entity::find()
        .filter(chapter_manga_prompt::Column::ProjectId.is_in(project_ids.iter().copied()))
        .filter(
            chapter_manga_prompt::Column::PromptType
                .is_in([MANGA_PAGE_PROMPT_TYPE, PIPELINE_STATUS_PROMPT_TYPE]),
        )
        .all(db)
        .await
        .map_err(|error| {
            ProjectProgressReadError::database("查询漫画页面与管线提示词失败", error)
        })?;
    let mut counts_by_project: HashMap<i64, ProgressCounts> = HashMap::new();
    let mut pipeline_by_project: HashMap<i64, PipelineProjectState> = HashMap::new();
    for row in rows {
        if row.prompt_type == MANGA_PAGE_PROMPT_TYPE {
            counts_by_project
                .entry(row.project_id)
                .or_default()
                .add(&row.status);
        } else if row.prompt_type == PIPELINE_STATUS_PROMPT_TYPE {
            let pipeline = pipeline_by_project.entry(row.project_id).or_default();
            add_pipeline_layout(pipeline, row.layout.as_ref());
        }
    }
    let mut progress = HashMap::with_capacity(project_ids.len());
    for project_id in project_ids {
        progress.insert(
            *project_id,
            comic_project_progress_from_counts(
                counts_by_project.remove(project_id).unwrap_or_default(),
                pipeline_by_project.remove(project_id).unwrap_or_default(),
            ),
        );
    }
    Ok(progress)
}

fn add_pipeline_layout(state: &mut PipelineProjectState, layout: Option<&Value>) {
    let Some(layout) = layout else {
        return;
    };
    let status = layout
        .get("pipeline_status")
        .and_then(Value::as_str)
        .map(str::trim)
        .unwrap_or_default();
    match status {
        "running" => state.running = true,
        "failed" => state.failed = true,
        "cancelled" => state.cancelled = true,
        _ => {}
    }
    if state.message.is_none() {
        state.message = layout
            .get("message")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|message| !message.is_empty())
            .map(str::to_string);
    }
}

fn comic_project_progress_from_counts(
    counts: ProgressCounts,
    pipeline: PipelineProjectState,
) -> ProjectProgressSummary {
    let total = non_zero(counts.total());
    let current = total.map(|_| counts.completed);
    if counts.failed > 0 || pipeline.failed {
        return project_progress(
            PROJECT_KIND_COMIC,
            "manga_failed",
            "漫画生成失败",
            current,
            total,
            ProjectProgressStatus::Failed,
            Some("存在失败的漫画流水线或页面生成记录".to_string()),
        );
    }
    if counts.running > 0 || pipeline.running {
        return project_progress(
            PROJECT_KIND_COMIC,
            "manga_running",
            "漫画生成中",
            current,
            total,
            ProjectProgressStatus::Running,
            pipeline
                .message
                .or_else(|| progress_count_detail("已完成页面", current, total)),
        );
    }
    if pipeline.cancelled {
        return project_progress(
            PROJECT_KIND_COMIC,
            "manga_cancelled",
            "漫画生成已取消",
            current,
            total,
            ProjectProgressStatus::Blocked,
            pipeline.message,
        );
    }
    if total.is_some() && counts.completed >= counts.total() {
        return project_progress(
            PROJECT_KIND_COMIC,
            "manga_ready",
            "漫画已生成",
            current,
            total,
            ProjectProgressStatus::Completed,
            progress_count_detail("已完成页面", current, total),
        );
    }
    if counts.completed > 0 {
        return project_progress(
            PROJECT_KIND_COMIC,
            "manga_partial",
            "漫画部分完成",
            current,
            total,
            ProjectProgressStatus::Idle,
            progress_count_detail("已完成页面", current, total),
        );
    }
    project_progress(
        PROJECT_KIND_COMIC,
        "manga_workspace",
        "漫画制作中",
        current,
        total,
        ProjectProgressStatus::Idle,
        progress_count_detail("页面", current, total),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn comic_project_progress_reports_running_pipeline_before_idle_counts() {
        let mut pipeline = PipelineProjectState {
            running: true,
            ..Default::default()
        };
        pipeline.message = Some("正在生成漫画页".to_string());
        let summary = comic_project_progress_from_counts(ProgressCounts::default(), pipeline);

        assert_eq!(summary.kind, PROJECT_KIND_COMIC);
        assert_eq!(summary.stage, "manga_running");
        assert_eq!(summary.label, "漫画生成中");
        assert_eq!(summary.status, ProjectProgressStatus::Running);
        assert_eq!(summary.detail.as_deref(), Some("正在生成漫画页"));
    }
}
