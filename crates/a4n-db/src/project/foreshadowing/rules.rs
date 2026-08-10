use a4n_core::models::character::foreshadowing_index;

use super::contracts::ForeshadowingAnalysisDraft;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ForeshadowingResolutionAction {
    Keep,
    Resolve,
}

pub(super) fn foreshadowing_matches_analysis_draft(
    existing: &foreshadowing_index::Model,
    draft: &ForeshadowingAnalysisDraft,
) -> bool {
    let expected_status = foreshadowing_status_after_setup_resync(
        existing.status.as_str(),
        existing.payoff_chapter,
        draft.status.as_str(),
    );
    existing.status == expected_status
        && existing.related_characters == draft.related_characters
        && existing.importance == draft.importance
}

pub(super) fn foreshadowing_status_after_setup_resync(
    existing_status: &str,
    payoff_chapter: Option<i32>,
    draft_status: &str,
) -> String {
    if payoff_chapter.is_some() || matches!(existing_status, "resolved" | "abandoned") {
        existing_status.to_string()
    } else {
        draft_status.to_string()
    }
}

pub(super) fn foreshadowing_resolution_action(
    existing: &foreshadowing_index::Model,
    chapter_number: i32,
) -> Result<ForeshadowingResolutionAction, String> {
    if existing.setup_chapter >= chapter_number {
        return Err(format!("埋设章节 {} 不早于回收章节", existing.setup_chapter));
    }
    match existing.payoff_chapter {
        Some(payoff_chapter) if payoff_chapter != chapter_number => {
            Err(format!("已经由第 {payoff_chapter} 章回收"))
        }
        Some(_) if existing.status == "resolved" => Ok(ForeshadowingResolutionAction::Keep),
        Some(_) => Err(format!("回收章节已存在，但状态为 {}", existing.status)),
        None if existing.status == "pending" || existing.status == "partially_revealed" => {
            Ok(ForeshadowingResolutionAction::Resolve)
        }
        None => Err(format!("当前状态为 {}", existing.status)),
    }
}
