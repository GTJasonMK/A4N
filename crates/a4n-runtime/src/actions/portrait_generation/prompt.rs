mod fingerprint;
mod generation;
mod preview;
mod request;
mod rules;

pub use generation::generate_portrait_prompt;
pub use preview::preview_generate_portrait_prompt_request;

pub(in crate::actions::portrait_generation) use request::{
    GeneratePortraitPromptRequestContext, build_generate_portrait_prompt_request,
};
pub(super) use rules::profile_portrait_seed_prompt;
#[cfg(test)]
pub(super) use rules::{
    DEFAULT_PORTRAIT_EXPRESSION, first_expression_or_default, optional_profile_text,
    portrait_profile_context,
};
