mod formatting;
mod query;

#[cfg(test)]
pub(in crate::chapter_generation::context_before) use formatting::{
    character_profile_context_line, character_state_context_line,
};
pub(in crate::chapter_generation::context_before) use query::recent_character_state_context_lines;
