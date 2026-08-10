//! 状态转换规则。

use a4n_domain::project::ProjectStatus;

/// 检查从前状态到后状态是否为合法前进转换
///
/// 前进转换：Draft → BlueprintReady → PartOutlinesReady → ChapterOutlinesReady → Writing → Completed
pub fn is_valid_forward(from: ProjectStatus, to: ProjectStatus) -> bool {
    valid_forward_targets(from).contains(&to)
}

/// 检查从前状态到后状态是否为合法回退转换
pub fn is_valid_rollback(from: ProjectStatus, to: ProjectStatus) -> bool {
    from.ordinal() > to.ordinal()
}

/// 获取前向转换的所有合法目标状态
pub fn valid_forward_targets(from: ProjectStatus) -> &'static [ProjectStatus] {
    match from {
        ProjectStatus::Draft => &[ProjectStatus::BlueprintReady],
        ProjectStatus::BlueprintReady => &[
            ProjectStatus::PartOutlinesReady,
            ProjectStatus::ChapterOutlinesReady,
        ],
        ProjectStatus::PartOutlinesReady => &[ProjectStatus::ChapterOutlinesReady],
        ProjectStatus::ChapterOutlinesReady => &[ProjectStatus::Writing],
        ProjectStatus::Writing => &[ProjectStatus::Completed],
        ProjectStatus::Completed => &[],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_forward_draft_to_blueprint() {
        assert!(is_valid_forward(
            ProjectStatus::Draft,
            ProjectStatus::BlueprintReady
        ));
    }

    #[test]
    fn test_invalid_forward_draft_to_writing() {
        assert!(!is_valid_forward(
            ProjectStatus::Draft,
            ProjectStatus::Writing
        ));
    }

    #[test]
    fn test_shortcut_blueprint_to_chapter_outlines() {
        assert!(is_valid_forward(
            ProjectStatus::BlueprintReady,
            ProjectStatus::ChapterOutlinesReady
        ));
    }

    #[test]
    fn test_rollback_writing_to_chapter_outlines_is_valid() {
        assert!(is_valid_rollback(
            ProjectStatus::Writing,
            ProjectStatus::ChapterOutlinesReady
        ));
    }

    #[test]
    fn rollback_legality_matches_cleanup_table() {
        for from in &ProjectStatus::ALL {
            for to in &ProjectStatus::ALL {
                if from == to {
                    continue;
                }
                let has_cleanup = !super::super::rollback::rollback_actions(*from, *to).is_empty();
                if has_cleanup {
                    assert!(
                        is_valid_rollback(*from, *to),
                        "{from:?}->{to:?} has cleanup actions but is not a valid rollback"
                    );
                }
                if is_valid_rollback(*from, *to) {
                    assert!(
                        has_cleanup
                            || (*from == ProjectStatus::Completed && *to == ProjectStatus::Writing),
                        "{from:?}->{to:?} is valid but has no cleanup mapping"
                    );
                }
            }
        }
    }

    #[test]
    fn test_rollback_draft_is_invalid() {
        assert!(!is_valid_rollback(
            ProjectStatus::Draft,
            ProjectStatus::Draft
        ));
    }
}
