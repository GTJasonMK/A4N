use super::RollbackImpactSummary;
use crate::{AppState, actions::app_error_message};
use a4n_core::{
    models::project::novel_project,
    state::{
        rollback::{CleanupAction, rollback_actions},
        status::ProjectStatus,
    },
};
use a4n_orchestrator::project::PROJECT_KIND_TEXT;
use a4n_orchestrator::project::ProjectRollbackImpactQuery;

pub(super) async fn project_record_by_id(
    state: &AppState,
    project_id: i64,
) -> Result<novel_project::Model, String> {
    state
        .orchestrator
        .project_by_id(project_id)
        .await
        .map_err(app_error_message)?
        .ok_or_else(|| format!("项目不存在: {project_id}"))
}

pub(super) fn ensure_text_project_status_command(
    project: &novel_project::Model,
) -> Result<(), String> {
    if project.project_kind != PROJECT_KIND_TEXT {
        return Err("非文本项目不能使用文本写作状态机".to_string());
    }
    Ok(())
}

pub(super) fn rollback_impact_count(
    impacts: &[RollbackImpactSummary],
    kind: &str,
) -> Result<i32, String> {
    let Some(impact) = impacts.iter().find(|impact| impact.kind == kind) else {
        return Ok(0);
    };
    impact
        .count
        .ok_or_else(|| format!("回退影响 {kind} 缺少数量"))
}

pub(super) fn rollback_path(from: ProjectStatus, to: ProjectStatus) -> Vec<String> {
    ProjectStatus::all()
        .iter()
        .rev()
        .filter(|status| status.ordinal() < from.ordinal() && status.ordinal() >= to.ordinal())
        .map(|status| status.as_str().to_string())
        .collect()
}

pub(super) async fn build_rollback_impacts(
    state: &AppState,
    project_id: i64,
    from: ProjectStatus,
    to: ProjectStatus,
) -> Result<Vec<RollbackImpactSummary>, String> {
    let actions = rollback_actions(from, to);
    state
        .orchestrator
        .project_rollback_impacts(rollback_impact_query(project_id, &actions))
        .await
        .map_err(app_error_message)
}

pub(super) fn rollback_impact_query(
    project_id: i64,
    actions: &[CleanupAction],
) -> ProjectRollbackImpactQuery {
    ProjectRollbackImpactQuery {
        project_id,
        delete_chapters: actions
            .iter()
            .any(|action| matches!(action, CleanupAction::DeleteChapters)),
        delete_chapter_outlines: actions
            .iter()
            .any(|action| matches!(action, CleanupAction::DeleteChapterOutlines)),
        delete_part_outlines: actions
            .iter()
            .any(|action| matches!(action, CleanupAction::DeletePartOutlines)),
        delete_blueprint_versions: actions
            .iter()
            .any(|action| matches!(action, CleanupAction::DeleteBlueprintVersions)),
    }
}

pub(super) fn build_rollback_summary(path: &[String], impacts: &[RollbackImpactSummary]) -> String {
    if path.is_empty() {
        return "无需回退。".to_string();
    }
    let mut lines = Vec::new();
    lines.push(format!("将执行回退路径：{}", path.join(" -> ")));
    for impact in impacts {
        let mut line = format!("- {}", impact.title);
        if let Some(count) = impact.count {
            line.push_str(&format!("：{}", count));
        }
        if let (Some(start), Some(end)) = (impact.chapter_start, impact.chapter_end) {
            line.push_str(&format!("（第 {}-{} 章）", start, end));
        }
        if let (Some(start), Some(end)) = (impact.part_start, impact.part_end) {
            line.push_str(&format!("（第 {}-{} 部分）", start, end));
        }
        lines.push(line);
        if let Some(note) = &impact.note {
            lines.push(format!("  {}", note));
        }
    }
    lines.push("注意：回退会删除依赖数据，此操作不可恢复。".to_string());
    lines.join("\n")
}
