use super::*;

use super::derived_fields::derive_chat_video;
use super::validation::{
    validate_prompt_has_no_upload_status, validate_prompt_has_shot_storyboard_section,
    validate_reference_ids,
};

mod contracts;
mod parser;
mod prompt_text;
mod validation;

use self::contracts::ChatVideoPromptResponse;
pub(in crate::video) use self::contracts::ChatVideoPromptShot;
use self::validation::{validate_chat_video_input_images, validate_chat_video_source_span_ids};
pub(in crate::video) use parser::{
    parse_chat_video_prompt_shots, validate_chat_video_prompt_shot_against_segment,
};
use prompt_text::*;

pub(super) fn chat_video_prompt_response_schema() -> schemars::Schema {
    schemars::schema_for!(ChatVideoPromptResponse)
}
