mod prelude {
    pub(super) use crate::{
        actions::{app_error_message, json_contract::reject_unknown_fields},
        state::AppState,
    };
    pub(super) use a4n_llm::provider::ChatRequest;
    pub(super) use a4n_llm::trace::context_aware_chat_request_trace_json;
    pub(super) use a4n_orchestrator::character::{
        CharacterSyncPersistenceInput, RecordCompletedGenerationRunInput,
    };
    pub(super) use serde_json::{Value, json};

    pub(super) use super::super::json_utils::{
        required_array, required_i32_at, required_string, required_string_array,
    };
    pub(super) use super::super::snapshot_storage::chapter_state_record_inputs;
    pub(super) use super::super::state_projection::pending_attribute_events;
    pub(super) use super::super::sync_implicit::{classify_behaviors, decide_implicit_reviews};
    pub(super) use super::super::sync_response::{
        CHARACTER_SYNC_ATTRIBUTE_CHANGES_PER_CHAPTER_MAX, CHARACTER_SYNC_BEHAVIORS_PER_CHAPTER_MAX,
        CHARACTER_SYNC_DELETION_CANDIDATES_PER_CHAPTER_MAX, ChapterStateDraft,
        chapter_state_drafts, validate_attribute_change, validate_behavior, validate_chapter_state,
        validate_deletion_candidate, validate_unique_deletion_candidates,
    };
    pub(super) use super::super::sync_types::{
        CHARACTER_PROFILE_SYNC_STEPS, CharacterProgressScope, CharacterSyncContext,
        CharacterSyncExtraction,
    };
    pub(super) use super::super::sync_validate::{
        sort_sync_items_by_chapter, validate_materialized_character_sync_projection,
        validate_sync_evidence_against_chapters, validate_sync_item_chapters,
        validate_sync_item_count_by_chapter,
    };
    pub(super) use super::super::sync_write::{
        BuildCharacterSyncWriteStepsInput, build_character_sync_write_step_records,
    };
}

mod api;
mod contracts;
mod parsing;
mod persistence;
mod request;
mod stages;

pub(crate) use api::character_sync_preview_steps;
pub use api::sync_character_profile_from_chapters;
pub(in crate::actions::character_ai) use api::{
    character_sync_preview_steps_for_context, sync_character_profile_context_with_scope,
    sync_character_profile_from_chapters_with_scope,
};
pub(in crate::actions::character_ai) use contracts::CharacterSyncStage;
#[cfg(test)]
pub(in crate::actions::character_ai) use request::character_sync_stage_contract;

use contracts::CharacterSyncStageOutputs;
