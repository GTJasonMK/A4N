mod contracts;
mod parser;
mod prompt_text;
mod validation;

pub(in crate::video) use self::contracts::ChatVideoPromptShot;
pub(in crate::video) use parser::{
    parse_chat_video_prompt_shots, validate_chat_video_prompt_shot_against_segment,
};
pub(in crate::video) use prompt_text::validate_chat_video_prompt_text;
