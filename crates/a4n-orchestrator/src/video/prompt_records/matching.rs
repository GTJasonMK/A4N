mod assets;
mod context;

#[cfg(test)]
use a4n_core::models::chapter::chapter_video_prompt;
#[cfg(test)]
use serde_json::Value;

#[cfg(test)]
use crate::video::{
    BOUNDARY_FRAME_PROMPT_CONTRACT_VERSION, CHAT_VIDEO_CONTRACT_VERSION,
    FIRST_FRAME_CHAIN_CONTRACT_VERSION, FRAME_SEGMENT_PLAN_CONTRACT_VERSION,
};
#[cfg(test)]
use a4n_core::prompt_language::PromptOutputLanguage;

#[cfg(test)]
use super::contracts::{PromptGenerationContext, PromptGenerationContextForWorkflow};

#[cfg(test)]
pub(super) use assets::db_video_prompt_update_input;
pub(in crate::video) use assets::video_generation_asset_fingerprint;
#[cfg(test)]
pub(super) use context::prompt_matches_generation_context;
pub(super) use context::prompt_matches_generation_context_for_workflow;
#[cfg(test)]
use context::reference_ids_match;
pub(in crate::video) use context::video_execution_segment_identity_matches;
pub(in crate::video) use context::video_execution_segment_lineage_matches;
pub(in crate::video) use context::{expected_contract_version, reusable_rows_are_complete};

#[cfg(test)]
include!("tests.rs");
