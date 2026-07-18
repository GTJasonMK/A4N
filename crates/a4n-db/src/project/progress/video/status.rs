use super::facts::VideoProgressFacts;
use crate::project::PROJECT_KIND_VIDEO;
use crate::project::progress::shared::{non_zero, progress_count_detail, project_progress};
use crate::project::progress::{ProjectProgressStatus, ProjectProgressSummary};

pub(super) fn video_project_progress_from_counts(
    facts: VideoProgressFacts,
) -> ProjectProgressSummary {
    let total = non_zero(facts.video_prompt_total.max(facts.video_counts.total()));
    let current = total.map(|_| facts.video_counts.completed);
    if facts.prompt_counts.failed > 0 || facts.video_counts.failed > 0 {
        return project_progress(
            PROJECT_KIND_VIDEO,
            "video_failed",
            "视频生成失败",
            current,
            total,
            ProjectProgressStatus::Failed,
            Some("存在当前失败的视频提示词或视频生成记录".to_string()),
        );
    }
    if facts.prompt_counts.running > 0
        || facts.video_counts.running > 0
        || facts.video_counts.pending > 0
    {
        return project_progress(
            PROJECT_KIND_VIDEO,
            "video_generating",
            "视频生成中",
            current,
            total,
            ProjectProgressStatus::Running,
            progress_count_detail("已完成视频片段", current, total),
        );
    }
    if let Some(total) = total
        && facts.video_counts.completed >= total
    {
        return project_progress(
            PROJECT_KIND_VIDEO,
            "video_ready",
            "视频已生成",
            current,
            Some(total),
            ProjectProgressStatus::Completed,
            progress_count_detail("已完成视频片段", current, Some(total)),
        );
    }
    if facts.video_counts.completed > 0 {
        return project_progress(
            PROJECT_KIND_VIDEO,
            "video_partial",
            "视频部分完成",
            current,
            total,
            ProjectProgressStatus::Idle,
            progress_count_detail("已完成视频片段", current, total),
        );
    }
    if facts.video_prompt_total > 0 {
        return project_progress(
            PROJECT_KIND_VIDEO,
            "video_ready_to_generate",
            "视频待生成",
            current,
            total,
            ProjectProgressStatus::Idle,
            progress_count_detail("视频提示词", Some(0), total),
        );
    }
    if facts.frame_prompt_total > 0 {
        return project_progress(
            PROJECT_KIND_VIDEO,
            "video_frame_prompting",
            "视频成图提示词制作中",
            None,
            Some(facts.frame_prompt_total),
            ProjectProgressStatus::Idle,
            Some(format!("已有 {} 条成图提示词", facts.frame_prompt_total)),
        );
    }
    if facts.segment_plan_total > 0 {
        return project_progress(
            PROJECT_KIND_VIDEO,
            "video_planning",
            "视频片段规划中",
            None,
            Some(facts.segment_plan_total),
            ProjectProgressStatus::Idle,
            Some(format!("已有 {} 条片段规划", facts.segment_plan_total)),
        );
    }
    project_progress(
        PROJECT_KIND_VIDEO,
        "video_workspace",
        "视频制作中",
        None,
        None,
        ProjectProgressStatus::Idle,
        None,
    )
}
