mod prompt_context;
mod records;
mod references;
mod rollback;
mod types;

pub use prompt_context::{portrait_named_prompt_profile_rows, portrait_prompt_profile_rows};
pub use records::{
    character_portrait_by_id, character_portrait_records_by_project,
    create_character_portrait_for_profile, primary_character_portrait_record,
    set_primary_character_portrait_record,
};
pub use references::{missing_portrait_profiles, portrait_reference_images_by_ids};
pub use rollback::rollback_failed_portrait_creation_records;
pub use types::{
    CharacterPortraitCreateInput, CharacterPortraitRecordError, CharacterPortraitRollbackError,
    CharacterPortraitRollbackResult, PortraitNamedPromptProfileRows, PortraitPromptProfileRows,
};

pub(super) use rollback::generated_image_matches_any_portrait;
#[cfg(test)]
use rollback::generated_image_matches_portrait;

#[cfg(test)]
use a4n_core::models::{ai::generated_image, character::character_portrait};
#[cfg(test)]
use serde_json::Value;

#[cfg(test)]
mod tests;
