use a4n_core::models::character::foreshadowing_index;

use super::contracts::ForeshadowingAnalysisDraft;

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
