mod builders;
mod context;
mod payload;

pub(crate) const IMPLICIT_UPDATE_PROMPT_NAME: &str = "implicit_update";

pub(crate) use builders::build_implicit_update_manifest_user_prompt;
pub(crate) use context::implicit_update_stable_context;

#[cfg(test)]
mod tests;
