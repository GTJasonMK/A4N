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
        .filter(|value| !json_value_is_empty(value))
        .or_else(|| draft.related_characters.clone())
}

fn json_value_is_empty(value: &serde_json::Value) -> bool {
    match value {
        serde_json::Value::Null => true,
        serde_json::Value::String(value) => value.trim().is_empty(),
        serde_json::Value::Array(value) => value.is_empty(),
        serde_json::Value::Object(value) => value.is_empty(),
        _ => false,
    }
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
