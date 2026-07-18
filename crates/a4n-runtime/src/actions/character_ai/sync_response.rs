use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

use super::json_utils::{
    optional_string, optional_string_or_null, required_array, required_attribute_change_operation,
    required_category, required_i32_at, required_string, required_string_array, required_value,
};

mod changes;
mod contracts;
mod fields;
mod state;

pub(super) use changes::{validate_attribute_change, validate_behavior};
#[cfg(test)]
pub(super) use contracts::parse_character_sync_output;
pub(super) use contracts::{
    CHARACTER_SYNC_ATTRIBUTE_CHANGES_PER_CHAPTER_MAX, CHARACTER_SYNC_BEHAVIORS_PER_CHAPTER_MAX,
    CHARACTER_SYNC_DELETION_CANDIDATES_PER_CHAPTER_MAX, ChapterStateDraft,
    validate_unique_deletion_candidates,
};
use contracts::{
    CHARACTER_SYNC_BEHAVIOR_TAGS_MAX_ITEMS, CHARACTER_SYNC_DESCRIPTION_MAX_CHARS,
    CHARACTER_SYNC_IMPACT_ANALYSIS_MAX_CHARS, CHARACTER_SYNC_RELATED_CHARACTERS_MAX_ITEMS,
    CHARACTER_SYNC_STATE_SUMMARY_MAX_CHARS,
};
use fields::{
    optional_limited_string_or_null, reject_legacy_anchor_field, required_limited_string,
    required_limited_string_array, required_positive_i64,
};
pub(super) use state::{chapter_state_drafts, validate_chapter_state, validate_deletion_candidate};
