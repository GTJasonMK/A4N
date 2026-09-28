use super::status::{
    WORKFLOW_STATUS_CANCELLED, WORKFLOW_STATUS_COMPLETED, WORKFLOW_STATUS_FAILED,
    WORKFLOW_STATUS_INTERRUPTED, WORKFLOW_STATUS_RUNNING,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnifiedResumeDispatch {
    VideoProviderJob { video_id: i64 },
    VideoCanvasGraph,
    ChapterAnalysis,
    ChapterContentSync { chapter_id: i64 },
    SceneProfileSync,
    BlueprintGeneration,
    BlueprintRefinement,
    ProjectFactFeedback { project_id: i64, rebuild: bool },
}

pub fn require_workflow_can_resume(
    status: &str,
    can_resume: bool,
    resume_block_reason: Option<&str>,
) -> Result<(), String> {
    match status {
        WORKFLOW_STATUS_RUNNING => return Err("工作流正在运行，不能重复恢复".to_string()),
        WORKFLOW_STATUS_COMPLETED => return Err("工作流已完成，不能恢复".to_string()),
        _ => {}
    }
    if can_resume {
        return if matches!(
            status,
            WORKFLOW_STATUS_FAILED | WORKFLOW_STATUS_CANCELLED | WORKFLOW_STATUS_INTERRUPTED
        ) {
            Ok(())
        } else {
            Err(format!("工作流状态 {status} 不支持恢复"))
        };
    }
    let reason = resume_block_reason
        .filter(|reason| !reason.trim().is_empty())
        .map(str::to_owned)
        .ok_or_else(|| format!("工作流状态 {status} 缺少不可恢复原因"))?;
    Err(reason)
}

pub fn resolve_unified_resume_dispatch(
    workflow_kind: &str,
    scope_table: Option<&str>,
    scope_id: Option<i64>,
) -> Result<UnifiedResumeDispatch, String> {
    match workflow_kind {
        "video_provider_job" => Ok(UnifiedResumeDispatch::VideoProviderJob {
            video_id: scope_id
                .filter(|_| scope_table == Some("generated_video"))
                .ok_or_else(|| "视频 provider 工作流缺少 generated_video 目标".to_string())?,
        }),
        "video_canvas_graph" => Ok(UnifiedResumeDispatch::VideoCanvasGraph),
        "chapter_analysis" => Ok(UnifiedResumeDispatch::ChapterAnalysis),
        "chapter_content_sync" => Ok(UnifiedResumeDispatch::ChapterContentSync {
            chapter_id: scope_id
                .filter(|id| *id > 0 && scope_table == Some("chapter"))
                .ok_or_else(|| "章节资料同步工作流缺少 chapter 目标".to_string())?,
        }),
        "scene_profile_sync" => {
            if scope_id
                .filter(|id| *id > 0 && scope_table == Some("novel_project"))
                .is_none()
            {
                return Err("场景档案同步工作流缺少 novel_project 目标".to_string());
            }
            Ok(UnifiedResumeDispatch::SceneProfileSync)
        }
        "blueprint_generation" => {
            if scope_id
                .filter(|id| *id > 0 && scope_table == Some("project"))
                .is_none()
            {
                return Err("蓝图生成工作流缺少 project 目标".to_string());
            }
            Ok(UnifiedResumeDispatch::BlueprintGeneration)
        }
        "blueprint_refine" => {
            if scope_id
                .filter(|id| *id > 0 && scope_table == Some("novel_blueprint"))
                .is_none()
            {
                return Err("蓝图优化工作流缺少 novel_blueprint 目标".to_string());
            }
            Ok(UnifiedResumeDispatch::BlueprintRefinement)
        }
        "project_fact_consolidation" | "project_fact_rebuild" => {
            Ok(UnifiedResumeDispatch::ProjectFactFeedback {
                project_id: scope_id
                    .filter(|id| *id > 0 && scope_table == Some("novel_project"))
                    .ok_or_else(|| "正文事实反馈缺少项目作用域".to_string())?,
                rebuild: workflow_kind == "project_fact_rebuild",
            })
        }
        "manga_pipeline" => Err("漫画流水线请在漫画章节页使用“续传流水线”入口恢复".to_string()),
        other => Err(format!("工作流类型 {other} 暂未接入统一恢复入口")),
    }
}

pub fn plan_unified_resume(
    status: &str,
    can_resume: bool,
    resume_block_reason: Option<&str>,
    workflow_kind: &str,
    scope_table: Option<&str>,
    scope_id: Option<i64>,
) -> Result<UnifiedResumeDispatch, String> {
    require_workflow_can_resume(status, can_resume, resume_block_reason)?;
    resolve_unified_resume_dispatch(workflow_kind, scope_table, scope_id)
}
