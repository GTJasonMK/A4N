//! 角色 AI 测试公共导入（避免污染 production 入口的 orch 透传壳统计）。

pub(super) use super::super::state_projection::fold_attribute_event_tuples;
pub(super) use a4n_api_contract::character::CharacterProfileSyncSkipReason;
pub(super) use a4n_db_models::models::chapter::chapter;
pub(super) use a4n_db_models::models::character::protagonist_profile;
pub(super) use a4n_domain::character::apply_attribute_to_state;
pub(super) use a4n_orchestrator::character::{
    CharacterSyncRange, CharacterSyncRangeMode, analysis_profile_sync_skip_reason,
    character_profile_skipped_result, character_sync_range, character_sync_start_chapter,
    profile_first_seen_chapter, profile_sync_skip_reason, validate_character_sync_target_chapter,
    validate_contiguous_character_sync_range,
};
pub(super) use a4n_orchestrator::character::{
    analysis_sync_character_names, character_names_from_chapter_analysis,
    character_names_from_discovery_result,
};
pub(super) use serde_json::json;
