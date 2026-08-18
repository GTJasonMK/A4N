use a4n_core::{
    json_contract::is_empty_json_value,
    models::character::foreshadowing_index,
};
use a4n_domain::character::ForeshadowingStatus;

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
    existing.related_characters
        == foreshadowing_related_characters_after_setup_resync(existing, draft)
}

pub(super) fn foreshadowing_related_characters_after_setup_resync(
    existing: &foreshadowing_index::Model,
    draft: &ForeshadowingAnalysisDraft,
) -> Option<serde_json::Value> {
    existing
        .related_characters
        .clone()
        .filter(|value| !is_empty_json_value(value))
        .or_else(|| draft.related_characters.clone())
}

pub(super) fn foreshadowing_resolution_action(
    existing: &foreshadowing_index::Model,
    chapter_number: i32,
) -> Result<ForeshadowingResolutionAction, String> {
    if existing.setup_chapter >= chapter_number {
        return Err(format!(
            "埋设章节 {} 不早于回收章节",
            existing.setup_chapter
        ));
    }
    let status = ForeshadowingStatus::parse(&existing.status)
        .ok_or_else(|| format!("当前状态非法：{}", existing.status))?;
    match existing.payoff_chapter {
        Some(payoff_chapter) if payoff_chapter != chapter_number => {
            Err(format!("已经由第 {payoff_chapter} 章回收"))
        }
        Some(_) if status == ForeshadowingStatus::Resolved => {
            Ok(ForeshadowingResolutionAction::Keep)
        }
        Some(_) => Err(format!("回收章节已存在，但状态为 {}", existing.status)),
        None if status.can_resolve() => Ok(ForeshadowingResolutionAction::Resolve),
        None => Err(format!("当前状态为 {}", existing.status)),
    }
}
