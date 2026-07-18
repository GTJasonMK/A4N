use super::shared::{ProgressCounts, non_zero, progress_count_detail, project_progress};
use super::{ProjectProgressReadError, ProjectProgressStatus, ProjectProgressSummary};
use crate::project::PROJECT_KIND_VIDEO;
use a4n_core::models::{ai::generated_video, canvas::canvas_graph};
use sea_orm::{
    ColumnTrait, DatabaseConnection, EntityTrait, FromQueryResult, QueryFilter, QuerySelect,
};
use std::collections::HashMap;

const FREEFORM_VIDEO_PROGRESS_TYPES: [&str; 4] = [
    "canvas_video",
    "canvas_reference_media",
    "canvas_trim",
    "canvas_assembly",
];

pub(super) async fn freeform_video_project_progress_by_project(
    db: &DatabaseConnection,
    project_ids: &[i64],
) -> Result<HashMap<i64, ProjectProgressSummary>, ProjectProgressReadError> {
    if project_ids.is_empty() {
        return Ok(HashMap::new());
    }
    let graph_counts = canvas_graph_counts_by_project(db, project_ids).await?;
    let videos = generated_video::Entity::find()
        .filter(generated_video::Column::ProjectId.is_in(project_ids.iter().copied()))
        .filter(generated_video::Column::VideoType.is_in(FREEFORM_VIDEO_PROGRESS_TYPES))
        .all(db)
        .await
        .map_err(|error| ProjectProgressReadError::database("查询自由画布生成视频失败", error))?;
    let mut counts_by_project: HashMap<i64, ProgressCounts> = HashMap::new();
    for video in videos {
        counts_by_project
            .entry(video.project_id)
            .or_default()
            .add(&video.status);
    }
    let mut progress = HashMap::with_capacity(project_ids.len());
    for project_id in project_ids {
        progress.insert(
            *project_id,
            freeform_video_project_progress_from_counts(
                graph_counts.get(project_id).copied().unwrap_or(0),
                counts_by_project.remove(project_id).unwrap_or_default(),
            ),
        );
    }
    Ok(progress)
}

#[derive(Debug, FromQueryResult)]
struct ProjectCountRow {
    project_id: i64,
    count: i64,
}

async fn canvas_graph_counts_by_project(
    db: &DatabaseConnection,
    project_ids: &[i64],
) -> Result<HashMap<i64, i64>, ProjectProgressReadError> {
    if project_ids.is_empty() {
        return Ok(HashMap::new());
    }
    let rows = canvas_graph::Entity::find()
        .select_only()
        .column(canvas_graph::Column::ProjectId)
        .column_as(canvas_graph::Column::Id.count(), "count")
        .filter(canvas_graph::Column::ProjectId.is_in(project_ids.iter().copied()))
        .group_by(canvas_graph::Column::ProjectId)
        .into_model::<ProjectCountRow>()
        .all(db)
        .await
        .map_err(|error| ProjectProgressReadError::database("统计自由画布图失败", error))?;
    Ok(rows
        .into_iter()
        .map(|row| (row.project_id, row.count.max(0)))
        .collect())
}

fn freeform_video_project_progress_from_counts(
    graph_count: i64,
    counts: ProgressCounts,
) -> ProjectProgressSummary {
    let total = non_zero(counts.total());
    let current = total.map(|_| counts.completed);
    if counts.failed > 0 {
        return project_progress(
            PROJECT_KIND_VIDEO,
            "video_canvas_failed",
            "画布视频生成失败",
            current,
            total,
            ProjectProgressStatus::Failed,
            Some("存在失败的画布视频产物".to_string()),
        );
    }
    if counts.running > 0 || counts.pending > 0 {
        return project_progress(
            PROJECT_KIND_VIDEO,
            "video_canvas_running",
            "画布视频生成中",
            current,
            total,
            ProjectProgressStatus::Running,
            progress_count_detail("已完成视频", current, total),
        );
    }
    if total.is_some() && counts.completed >= counts.total() {
        return project_progress(
            PROJECT_KIND_VIDEO,
            "video_canvas_ready",
            "画布视频已生成",
            current,
            total,
            ProjectProgressStatus::Completed,
            progress_count_detail("已完成视频", current, total),
        );
    }
    if counts.completed > 0 {
        return project_progress(
            PROJECT_KIND_VIDEO,
            "video_canvas_partial",
            "画布视频部分完成",
            current,
            total,
            ProjectProgressStatus::Idle,
            progress_count_detail("已完成视频", current, total),
        );
    }
    project_progress(
        PROJECT_KIND_VIDEO,
        "video_canvas_workspace",
        "视频画布制作中",
        None,
        non_zero(graph_count),
        ProjectProgressStatus::Idle,
        None,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn freeform_video_progress_prioritizes_failures_and_preserves_workspace_count() {
        let failed = freeform_video_project_progress_from_counts(
            2,
            ProgressCounts {
                completed: 1,
                failed: 1,
                ..Default::default()
            },
        );
        assert_eq!(failed.stage, "video_canvas_failed");
        assert_eq!(failed.status, ProjectProgressStatus::Failed);

        let workspace = freeform_video_project_progress_from_counts(3, ProgressCounts::default());
        assert_eq!(workspace.stage, "video_canvas_workspace");
        assert_eq!(workspace.current, None);
        assert_eq!(workspace.total, Some(3));
        assert_eq!(workspace.status, ProjectProgressStatus::Idle);
    }
}
