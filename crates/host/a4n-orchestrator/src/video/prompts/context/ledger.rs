use a4n_prompt::rendering::PromptRenderer;

use a4n_domain::source_text_hash as source_content_sha256;
use a4n_llm_context::LlmContextManifest;
use a4n_llm_context::{COMPRESSION_FULL_RAW, LlmContextLedger, LlmContextLedgerPackDraft};

use super::super::super::ChapterVideoSourceContext;

mod contracts;
mod dependencies;
mod manifest;
mod output_packs;
mod source_packs;

use contracts::{VIDEO_CONTEXT_LEDGER_VERSION, VIDEO_CONTEXT_SCOPE};
use dependencies::record_prompt_dependencies;
use output_packs::{add_optional_value_pack, add_previous_output_pack};
use source_packs::add_chapter_source_pack;

pub(in crate::video) use contracts::{
    VIDEO_PREVIOUS_OUTPUT_SHOOTING_SCRIPT, VideoContextManifestInput, VideoContextPreviousOutput,
};
pub(in crate::video) use manifest::{
    build_final_video_context_manifest, build_video_context_manifest,
};

#[cfg(test)]
mod tests;
