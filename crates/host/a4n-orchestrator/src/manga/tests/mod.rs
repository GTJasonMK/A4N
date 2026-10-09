use super::pipeline::prompt::{
    build_manga_step_user_message, manga_page_drawing_prompt_page_input, manga_page_slots_input,
};
use serde_json::json;

mod context;
mod messages;
mod validation;
