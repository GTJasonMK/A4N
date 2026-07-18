use std::collections::BTreeSet;

use serde_json::Value;

use crate::actions::app_error_message;
use crate::actions::character_implicit_review::BehaviorForReview;
use crate::state::AppState;

use super::super::super::json_utils::{
    required_category, required_i32, required_string, required_value,
};

pub(super) async fn behavior_records_for_review(
    state: &AppState,
    profile_id: i64,
    start_chapter: i32,
    behaviors: &[Value],
) -> Result<Vec<BehaviorForReview>, String> {
    let mut pending = behaviors
        .iter()
        .enumerate()
        .map(|(index, behavior)| behavior_for_review_from_sync(index, behavior))
        .collect::<Result<Vec<_>, _>>()?;
    pending.sort_by(|left, right| {
        right
            .chapter_number
            .cmp(&left.chapter_number)
            .then_with(|| left.behavior_id.cmp(&right.behavior_id))
    });

    let existing = state
        .orchestrator
        .behavior_records_before_chapter(profile_id, start_chapter)
        .await
        .map_err(app_error_message)?;

    pending.extend(existing.iter().map(BehaviorForReview::from));
    Ok(pending)
}

fn behavior_for_review_from_sync(
    index: usize,
    behavior: &Value,
) -> Result<BehaviorForReview, String> {
    let behavior_id = -((index as i64) + 1);
    Ok(BehaviorForReview {
        behavior_id,
        chapter_number: required_i32(behavior, "chapter_number")?,
        behavior_type: required_string(behavior, "behavior_type")?,
        behavior_description: required_string(behavior, "behavior_description")?,
        original_text: required_string(behavior, "original_text")?,
        behavior_tags: required_value(behavior, "behavior_tags")?.clone(),
        classification_results: required_value(behavior, "classification_results")?.clone(),
        created_at: None,
    })
}

pub(super) fn implicit_deletion_candidate_keys(
    deletion_candidates: &[Value],
) -> Result<BTreeSet<String>, String> {
    let mut keys = BTreeSet::new();
    for candidate in deletion_candidates {
        let category = required_category(candidate, "category")?;
        if category == "implicit" {
            keys.insert(required_string(candidate, "key")?);
        }
    }
    Ok(keys)
}
