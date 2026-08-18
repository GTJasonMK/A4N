use super::shared::{ProgressCounts, progress_count_detail, project_progress};
use super::{ProjectProgressReadError, ProjectProgressSummary};
use a4n_core::models::ai::generated_video;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};
use std::collections::HashMap;

const FREEFORM_VIDEO_PROGRESS_TYPES: [&str; 3] =
    [
        generated_video::VIDEO_TYPE_CANVAS,
        generated_video::VIDEO_TYPE_TRIMMED_CLIP,
        generated_video::VIDEO_TYPE_CANVAS_ASSEMBLY,
    ];

pub(super) async fn freeform_video_project_progress_by_project(
    db: &DatabaseConnection,
    project_ids: &[i64],
) -> Result<HashMap<i64, ProjectProgressSummary>, ProjectProgressReadError> {
    if project_ids.is_empty() {
        return Ok(HashMap::new());
    }
    let videos = generated_video::Entity::find()
        .filter(generated_video::Column::ProjectId.is_in(project_ids.iter().copied()))
        .filter(generated_video::Column::VideoType.is_in(FREEFORM_VIDEO_PROGRESS_TYPES))
        .all(db)
        .await
        .map_err(|error| ProjectProgressReadError::database("查询自由画布生成视频失败", error))?;
    let mut counts_by_project: HashMap<i64, ProgressCounts> = HashMap::new();
    for video in videos {
        counts_by_project.entry(video.project_id).or_default().add(
            "generated_video",
            video.id,
            &video.status,
        )?;
    }
    let mut progress = HashMap::with_capacity(project_ids.len());
    for project_id in project_ids {
        progress.insert(
            *project_id,
            freeform_video_project_progress_from_counts(
                counts_by_project.remove(project_id).unwrap_or_default(),
            ),
        );
    }
    Ok(progress)
}

fn freeform_video_project_progress_from_counts(counts: ProgressCounts) -> ProjectProgressSummary {
    let total = (counts.total() > 0).then_some(counts.total());
    let current = total.map(|_| counts.completed);
    if counts.failed > 0 {
        return project_progress(
            "画布视频生成失败",
            Some("存在失败的画布视频产物".to_string()),
        );
    }
    if counts.running > 0 || counts.pending > 0 {
        return project_progress(
            "画布视频生成中",
            progress_count_detail("已完成视频", current, total),
        );
    }
    if total.is_some() && counts.completed >= counts.total() {
        return project_progress(
            "画布视频已生成",
            progress_count_detail("已完成视频", current, total),
        );
    }
    if counts.completed > 0 {
        return project_progress(
            "画布视频部分完成",
            progress_count_detail("已完成视频", current, total),
        );
    }
    project_progress("视频画布制作中", None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn freeform_video_progress_prioritizes_failures() {
        let failed = freeform_video_project_progress_from_counts(ProgressCounts {
            completed: 1,
            failed: 1,
            ..Default::default()
        });
        assert_eq!(failed.label, "画布视频生成失败");

        let workspace = freeform_video_project_progress_from_counts(ProgressCounts::default());
        assert_eq!(workspace.label, "视频画布制作中");
        assert_eq!(workspace.detail, None);
    }
}
