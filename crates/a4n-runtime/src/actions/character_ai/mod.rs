mod character_discovery;
mod character_discovery_flow;
mod contracts;
mod implicit_classification;
mod json_utils;
mod preview;
mod progress;
mod scope;
mod snapshot_storage;
mod state_projection;
mod sync_batch;
mod sync_context;
mod sync_implicit;
mod sync_llm;
mod sync_preflight;
mod sync_profile;
mod sync_response;
mod sync_scope;
mod sync_trace;
mod sync_types;
mod sync_validate;
mod sync_write;
mod sync_write_records;

pub(crate) use character_discovery_flow::character_discovery_preview_steps;
pub use character_discovery_flow::discover_character_profiles_for_chapters;
pub use contracts::{CharacterAiPreviewStep, CharacterAiTaskScope};
pub use preview::{
    preview_discover_character_profiles_request, preview_sync_all_character_profiles_request,
    preview_sync_character_from_chapters_request,
};
pub use progress::{CharacterAiProgress, CharacterAiProgressReporter};
pub use scope::{
    character_discovery_task_scope, character_profile_sync_all_task_scope,
    character_profile_sync_task_scope,
};
pub use sync_batch::sync_all_character_profiles_to_chapter;
pub(crate) use sync_batch::{
    sync_all_character_profiles_preview_steps, sync_analysis_character_profiles_to_chapter,
};
pub(crate) use sync_preflight::preflight_analysis_character_profile_sync_to_chapter;
pub(crate) use sync_profile::character_sync_preview_steps;
pub use sync_profile::sync_character_profile_from_chapters;
pub use sync_types::CHARACTER_PROFILE_SYNC_STEPS;

#[cfg(test)]
use a4n_core::models::chapter::chapter;
#[cfg(test)]
use a4n_core::models::character::{attribute_event, protagonist_profile};
#[cfg(test)]
use character_discovery::{
    build_character_discovery_user_prompt, discovered_character_base_attributes,
    parse_character_discovery_output,
};
#[cfg(test)]
use character_discovery_flow::{
    select_character_discovery_chapters, validate_discovered_character_chapter_scope,
};
#[cfg(test)]
use implicit_classification::{
    apply_implicit_classification_to_behavior, build_implicit_classification_prompt,
    parse_implicit_classification_output,
};
#[cfg(test)]
use json_utils::attribute_counts;
#[cfg(test)]
use serde_json::{Value, json};
#[cfg(test)]
use snapshot_storage::attribute_events_at_chapter;
#[cfg(test)]
use state_projection::pending_attribute_events;
#[cfg(test)]
use state_projection::{
    apply_attribute_to_state, fold_attribute_event_tuples, project_attribute_changes,
};
#[cfg(test)]
use sync_context::character_llm_chapter_context;
#[cfg(test)]
use sync_llm::ensure_character_sync_response_not_truncated;
#[cfg(test)]
use sync_response::{
    chapter_state_drafts, parse_character_sync_output, validate_unique_deletion_candidates,
};
#[cfg(test)]
use sync_scope::{
    CharacterSyncRange, analysis_profile_sync_skip_reason, analysis_sync_character_names,
    character_names_from_chapter_analysis, character_names_from_discovery_result,
    character_sync_range, character_sync_start_chapter, profile_first_seen_chapter,
    sync_skip_summary, validate_character_sync_target_chapter,
};
#[cfg(test)]
use sync_scope::{CharacterSyncRangeMode, profile_sync_skip_reason};
#[cfg(test)]
use sync_types::CharacterSyncContext;
#[cfg(test)]
use sync_validate::{
    sort_sync_items_by_chapter, validate_contiguous_sync_range,
    validate_sync_evidence_against_chapters, validate_sync_item_chapters,
};
#[cfg(test)]
use sync_write::AttributeEventDraft;
#[cfg(test)]
use sync_write::{SyncWriteStepKind, sync_write_steps};

#[cfg(test)]
mod tests;
