//! 文本项目状态回退预览的纯规则（无 DB；影响查询由调用方注入结果）。

use a4n_core::{
    models::project::novel_project,
    state::{
        rollback::{CleanupAction, rollback_actions},
        status::ProjectStatus,
    },
};

use super::super::{PROJECT_KIND_TEXT, ProjectRollbackImpactQuery, ProjectRollbackImpactSummary};

/// Host 侧：非文本项目不能使用文本写作状态机。
pub fn ensure_text_project_status_command(project: &novel_project::Model) -> Result<(), String> {
    if project.project_kind != PROJECT_KIND_TEXT {
        Err("非文本项目不能使用文本写作状态机".to_string())
    } else {
        Ok(())
    }
}

/// 回退路径上的状态序列（不含起点当前状态）。
pub fn rollback_path(from: ProjectStatus, to: ProjectStatus) -> Vec<String> {
    ProjectStatus::all()
        .iter()
        .rev()
        .filter(|status| status.ordinal() < from.ordinal() && status.ordinal() >= to.ordinal())
        .map(|status| status.as_str().to_string())
        .collect()
}

/// 将 CleanupAction 列表映射为 DB 影响查询。
fn rollback_impact_query(project_id: i64, actions: &[CleanupAction]) -> ProjectRollbackImpactQuery {
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

/// 由 from/to 生成影响查询（封装 rollback_actions）。
pub fn rollback_impact_query_for_transition(
    project_id: i64,
    from: ProjectStatus,
    to: ProjectStatus,
) -> ProjectRollbackImpactQuery {
    let actions = rollback_actions(from, to);
    rollback_impact_query(project_id, &actions)
}

/// 从影响列表取某 kind 的数量；缺席为 0，在场但无 count 为错误。
pub fn rollback_impact_count(
    impacts: &[ProjectRollbackImpactSummary],
    kind: &str,
) -> Result<i32, String> {
    let Some(impact) = impacts.iter().find(|impact| impact.kind == kind) else {
        return Ok(0);
    };
    impact
        .count
        .ok_or_else(|| format!("回退影响 {kind} 缺少数量"))
}

/// 生成回退预览说明文本。
pub fn build_rollback_summary(path: &[String], impacts: &[ProjectRollbackImpactSummary]) -> String {
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

/// Host 预览路径：当前状态 + 回退路径状态序列。
pub fn rollback_preview_path(
    current_status: impl Into<String>,
    from: ProjectStatus,
    to: ProjectStatus,
) -> Vec<String> {
    let mut path = vec![current_status.into()];
    path.extend(rollback_path(from, to));
    path
}

/// 有删除影响时返回不可恢复警告文案。
pub fn rollback_preview_warning_message(
    impacts: &[ProjectRollbackImpactSummary],
) -> Option<String> {
    if impacts.is_empty() {
        None
    } else {
        Some("回退会删除依赖数据，此操作不可恢复。".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use a4n_core::state::rollback::CleanupAction;

    fn impact(kind: &str, count: Option<i32>) -> ProjectRollbackImpactSummary {
        ProjectRollbackImpactSummary {
            kind: kind.to_string(),
            title: kind.to_string(),
            tables: Vec::new(),
            count,
            chapter_start: None,
            chapter_end: None,
            part_start: None,
            part_end: None,
            note: None,
        }
    }

    #[test]
    fn rollback_impact_count_returns_zero_only_when_impact_is_absent() {
        let impacts = vec![impact("delete_chapters", None)];
        assert_eq!(
            rollback_impact_count(&impacts, "delete_chapter_outlines"),
            Ok(0)
        );
        let err = rollback_impact_count(&impacts, "delete_chapters").unwrap_err();
        assert!(err.contains("回退影响 delete_chapters 缺少数量"));
    }

    #[test]
    fn rollback_impact_count_accepts_explicit_count() {
        let impacts = vec![impact("delete_chapters", Some(3))];
        assert_eq!(rollback_impact_count(&impacts, "delete_chapters"), Ok(3));
    }

    #[test]
    fn rollback_impact_query_maps_cleanup_actions() {
        let query = rollback_impact_query(
            7,
            &[
                CleanupAction::DeleteChapters,
                CleanupAction::DeleteBlueprintVersions,
            ],
        );
        assert_eq!(query.project_id, 7);
        assert!(query.delete_chapters);
        assert!(!query.delete_chapter_outlines);
        assert!(!query.delete_part_outlines);
        assert!(query.delete_blueprint_versions);
    }

    #[test]
    fn rollback_preview_path_prefixes_current_status() {
        let path = rollback_preview_path(
            "writing",
            ProjectStatus::Writing,
            ProjectStatus::BlueprintReady,
        );
        assert_eq!(path.first().map(String::as_str), Some("writing"));
        assert!(path.len() >= 2);
        assert_eq!(path.last().map(String::as_str), Some("blueprint_ready"));
    }

    #[test]
    fn rollback_preview_warning_only_when_impacts_present() {
        assert_eq!(rollback_preview_warning_message(&[]), None);
        let warning = rollback_preview_warning_message(&[impact("delete_chapters", Some(1))])
            .expect("warning");
        assert!(warning.contains("不可恢复"));
    }
}
