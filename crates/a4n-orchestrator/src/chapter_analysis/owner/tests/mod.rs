use a4n_domain::source_spans::{source_span_prompt_values, source_spans_for_chapters};
use a4n_domain::source_text_hash;
use a4n_error::AppError;
use serde_json::{Value, json};

use super::materialize_analysis_source_spans;
use super::normalize_analysis;
use super::prompt::{build_chapter_analysis_stage_user_message, chapter_display_title};

mod support;
pub use support::{sample_analysis, sample_chapter};
mod contracts;
mod materialize;
mod normalization;
mod prompt;
