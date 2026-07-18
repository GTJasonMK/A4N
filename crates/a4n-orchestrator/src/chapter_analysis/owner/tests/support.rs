use a4n_core::models::chapter::chapter;
use a4n_core::source_spans::{
    source_span_prompt_values, source_spans_for_chapters, source_text_hash,
};
use chrono::NaiveDate;
use serde_json::json;

use super::normalization::normalize_analysis;
use super::prompt::{build_chapter_analysis_stage_user_message, required_chapter_title};
use super::*;

include!("support/chapter.rs");
include!("support/analysis.rs");
