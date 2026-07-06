//! 回退系统：按状态序号生成清理动作。

use super::status::ProjectStatus;

/// 回退时的清理动作
///
/// 每个变体描述回退时需要删除或清理的数据。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CleanupAction {
    /// 删除已生成章节
    DeleteChapters,
    /// 删除章节大纲
    DeleteChapterOutlines,
    /// 删除分部大纲
    DeletePartOutlines,
    /// 删除蓝图版本
    DeleteBlueprintVersions,
}

/// 回退动作映射：(from, to) → 清理动作列表
///
/// 共 15 条回退边，按后退距离递增排列。
pub fn rollback_actions(from: ProjectStatus, to: ProjectStatus) -> Vec<CleanupAction> {
    use ProjectStatus::*;

    if from.ordinal() <= to.ordinal() {
        return Vec::new();
    }

    let mut actions = Vec::new();
    let deletes_chapters = from.ordinal() >= Writing.ordinal() && to.ordinal() < Writing.ordinal();
    if deletes_chapters {
        actions.push(CleanupAction::DeleteChapters);
    }
    if from.ordinal() >= PartOutlinesReady.ordinal()
        && to.ordinal() < ChapterOutlinesReady.ordinal()
    {
        actions.push(CleanupAction::DeleteChapterOutlines);
    }
    if from.ordinal() >= PartOutlinesReady.ordinal() && to.ordinal() < PartOutlinesReady.ordinal() {
        actions.push(CleanupAction::DeletePartOutlines);
    }
    if to == Draft {
        actions.push(CleanupAction::DeleteBlueprintVersions);
    }
    actions
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_total_mapped_edges() {
        // 14 条回退边有清理动作，(Completed, Writing) 也是合法回退但无清理
        let all_statuses = ProjectStatus::all();
        let mut edge_count = 0;
        for from in all_statuses {
            for to in all_statuses {
                if from != to && !rollback_actions(*from, *to).is_empty() {
                    edge_count += 1;
                }
            }
        }
        assert_eq!(edge_count, 14, "应有 14 条带清理动作的回退边");
    }

    #[test]
    fn test_writing_to_draft_cleans_everything() {
        let actions = rollback_actions(ProjectStatus::Writing, ProjectStatus::Draft);
        assert_eq!(actions.len(), 4, "回退到草稿应清理全部 4 类状态数据");
    }

    #[test]
    fn test_completed_to_writing_preserves_data() {
        let actions = rollback_actions(ProjectStatus::Completed, ProjectStatus::Writing);
        assert!(actions.is_empty(), "完成→写作中不应清理数据");
    }

    #[test]
    fn test_forward_is_not_rollback() {
        // 前进方向不应有清理动作
        let actions = rollback_actions(ProjectStatus::Draft, ProjectStatus::BlueprintReady);
        assert!(actions.is_empty(), "前进转换不应是回退边");
    }
}
