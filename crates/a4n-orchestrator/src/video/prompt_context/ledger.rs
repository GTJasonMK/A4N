use a4n_prompt::rendering::PromptRenderer;

use a4n_core::source_spans::source_text_hash as source_content_sha256;
use a4n_core::source_spans::{SourceSpan, source_span_prompt_values};
use a4n_llm::context::LlmContextManifest;
use a4n_llm::context::{
    COMPRESSION_COMPRESSED_SCENE, COMPRESSION_FOCUSED_RAW, COMPRESSION_FULL_RAW,
    COMPRESSION_MINIMAL, LlmContextLedger, LlmContextLedgerPackDraft,
};

use super::super::ChapterVideoSourceContext;
use super::final_video_prompt_public_context;

mod contracts;
mod manifest;
mod output_packs;
mod rule_packs;
mod source_packs;

use contracts::{VIDEO_BUILDER_RULES_TEMPLATE, VIDEO_CONTEXT_LEDGER_VERSION, VIDEO_CONTEXT_SCOPE};
use output_packs::{add_optional_value_pack, add_previous_output_pack, trimmed_optional};
use rule_packs::{add_builder_rules_pack, add_prompt_rule_hash_pack, add_prompt_rule_packs};
use source_packs::{add_chapter_source_pack, add_public_chapter_source_pack};

pub(in crate::video) use contracts::{
    VIDEO_PREVIOUS_OUTPUT_SEGMENT_PLAN, VIDEO_PREVIOUS_OUTPUT_SHOOTING_SCRIPT,
    VideoContextManifestInput, VideoContextPreviousOutput,
};
pub(in crate::video) use manifest::{
    build_final_video_context_manifest, build_video_context_manifest,
};

#[cfg(test)]
mod tests;
