use crate::state::ServerState;
use serde_json::Value;

use super::super::super::{ApiError, portrait_generation_actions, to_value};
use super::input::{
    age_variant_input, generate_portrait_input, missing_portraits_input, regenerate_portrait_input,
};
use super::task::portrait_web_preview_command;

portrait_web_preview_command!(
    preview_generate_portrait_request,
    |args| generate_portrait_input(args, "preview_generate_portrait_request")?,
    portrait_generation_actions::preview_generate_portrait_request,
);

portrait_web_preview_command!(
    preview_regenerate_portrait_request,
    |args| regenerate_portrait_input(args, "preview_regenerate_portrait_request")?,
    portrait_generation_actions::preview_regenerate_portrait_request,
);

portrait_web_preview_command!(
    preview_generate_age_variant_request,
    |args| age_variant_input(args, "preview_generate_age_variant_request")?,
    portrait_generation_actions::preview_generate_age_variant_request,
);

portrait_web_preview_command!(
    preview_generate_missing_portraits_request,
    |args| missing_portraits_input(args)?,
    portrait_generation_actions::preview_generate_missing_portraits_request,
);
