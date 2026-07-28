use a4n_core::error::AppError;
use a4n_core::source_spans::{
    source_span_prompt_values, source_spans_for_chapters, source_text_hash,
};
use serde_json::{Value, json};

use super::materialize_analysis_source_spans;
use super::normalize_analysis;
use super::prompt::{build_chapter_analysis_stage_user_message, required_chapter_title};

mod support;
pub use support::{sample_analysis, sample_chapter};
mod contracts;
mod materialize;
mod normalization;
mod prompt;
