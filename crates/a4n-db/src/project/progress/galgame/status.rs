use crate::project::PROJECT_KIND_GALGAME;
use crate::project::progress::shared::{PipelineProjectState, non_zero, project_progress};
use crate::project::progress::{ProjectProgressStatus, ProjectProgressSummary};

pub(super) fn galgame_project_progress_from_counts(
    script_exists: bool,
    scene_count: i64,
    line_count: i64,
    pipeline: PipelineProjectState,
) -> ProjectProgressSummary {
    let total = non_zero(line_count);
    let current = total.map(|_| line_count);
    if pipeline.failed {
        return project_progress(
            PROJECT_KIND_GALGAME,
            "galgame_pipeline_failed",
            "Galgame 管线失败",
            current,
            total,
            ProjectProgressStatus::Failed,
            pipeline
                .message
                .or_else(|| Some("最近一次管线失败".to_string())),
        );
    }
    if pipeline.running {
        return project_progress(
            PROJECT_KIND_GALGAME,
            "galgame_pipeline_running",
            "Galgame 生成中",
            current,
            total,
            ProjectProgressStatus::Running,
            pipeline
                .message
                .or_else(|| Some("Galgame 管线正在运行".to_string())),
        );
    }
    if pipeline.cancelled {
        return project_progress(
            PROJECT_KIND_GALGAME,
            "galgame_pipeline_cancelled",
            "Galgame 管线已取消",
            current,
            total,
            ProjectProgressStatus::Blocked,
            pipeline.message,
        );
    }
    if !script_exists {
        return project_progress(
            PROJECT_KIND_GALGAME,
            "galgame_empty",
            "Galgame 待生成",
            Some(0),
            None,
            ProjectProgressStatus::Idle,
            Some("尚未生成剧本".to_string()),
        );
    }
    if line_count <= 0 {
        return project_progress(
            PROJECT_KIND_GALGAME,
            "galgame_script_empty",
            "Galgame 剧本为空",
            Some(0),
            Some(scene_count.max(0)),
            ProjectProgressStatus::Blocked,
            Some("已创建剧本但没有行内容".to_string()),
        );
    }
    project_progress(
        PROJECT_KIND_GALGAME,
        "galgame_script_ready",
        "Galgame 剧本已生成",
        Some(line_count),
        Some(line_count),
        ProjectProgressStatus::Completed,
        Some(format!("{} 场，{} 行", scene_count, line_count)),
    )
}
