mod attributes;
mod identity;
mod parsing;
mod prompt;

pub(super) use attributes::discovered_character_base_attributes;
pub(super) use identity::{
    character_identity_key, discovered_character_keys, existing_character_keys,
    existing_profile_for_prompt, profile_character_keys, profile_for_result,
};
pub(super) use parsing::parse_character_discovery_output;
pub(super) use prompt::{
    build_character_discovery_manifest_user_prompt, character_discovery_stable_context,
};

#[cfg(test)]
pub(super) use prompt::build_character_discovery_user_prompt;
