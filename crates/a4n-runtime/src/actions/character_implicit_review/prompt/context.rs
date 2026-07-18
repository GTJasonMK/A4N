use serde_json::json;

use crate::actions::character_implicit_review::types::ImplicitAttributeReviewSummary;

pub(crate) fn implicit_update_stable_context(
    profile_name: &str,
    attribute_key: &str,
    current_value: &serde_json::Value,
    stats: &ImplicitAttributeReviewSummary,
    threshold: usize,
    window_chapters: usize,
) -> serde_json::Value {
    json!({
        "profile_name": profile_name,
        "attribute_key": attribute_key,
        "current_value": current_value,
        "review_window": {
            "window_chapters": window_chapters,
            "non_conform_threshold": threshold,
        },
        "statistics": {
            "total": stats.total,
            "conform_count": stats.conform_count,
            "non_conform_count": stats.non_conform_count,
            "conform_rate": stats.conform_rate,
            "threshold_reached": stats.threshold_reached,
        },
        "evidence": implicit_update_prompt_evidence(stats),
    })
}

fn implicit_update_prompt_evidence(
    stats: &ImplicitAttributeReviewSummary,
) -> Vec<serde_json::Value> {
    stats
        .evidence
        .iter()
        .map(|item| {
            json!({
                "behavior_id": item.behavior_id,
                "chapter_number": item.chapter_number,
                "behavior_type": item.behavior_type,
                "behavior_description": item.behavior_description,
                "original_text": item.original_text,
                "behavior_tags": item.behavior_tags,
                "classification": item.classification,
            })
        })
        .collect()
}
