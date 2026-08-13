use crate::project::progress::ProjectProgressSummary;
use crate::project::progress::shared::{PipelineProjectState, project_progress};

pub(super) fn galgame_project_progress_from_counts(
    script_exists: bool,
    scene_count: i64,
    line_count: i64,
    pipeline: PipelineProjectState,
) -> ProjectProgressSummary {
    if pipeline.failed {
        return project_progress("Galgame 管线失败", pipeline.message);
    }
    if pipeline.running {
        return project_progress("Galgame 生成中", pipeline.message);
    }
    if pipeline.cancelled {
        return project_progress("Galgame 管线已取消", pipeline.message);
    }
    if !script_exists {
        return project_progress("Galgame 待生成", None);
    }
    if line_count <= 0 {
        return project_progress(
            "Galgame 剧本为空",
            Some(format!("已创建 {scene_count} 个场景但没有行内容")),
        );
    }
    project_progress(
        "Galgame 剧本已生成",
        Some(format!("{} 场，{} 行", scene_count, line_count)),
    )
}
