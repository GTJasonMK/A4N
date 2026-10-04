mod prompt_context;
mod record_mapping;
mod records;
mod references;
mod rollback;
mod types;

pub use prompt_context::{portrait_named_prompt_profile_rows, portrait_prompt_profile_rows};
pub(crate) use record_mapping::character_portrait_record_from_model;
pub use records::{
    character_portrait_by_id, character_portrait_record_by_id,
    character_portrait_records_by_project, character_portrait_records_by_project_records,
    create_character_portrait_for_profile, create_character_portrait_record_for_profile,
    primary_character_portrait_record, primary_character_portrait_record_value,
    set_primary_character_portrait_record,
};
pub use references::{
    missing_portrait_profile_records, missing_portrait_profiles, portrait_reference_images_by_ids,
};
pub use rollback::rollback_failed_portrait_creation_records;
pub use types::{
    CharacterPortraitRecordError, CharacterPortraitRollbackError, CharacterPortraitRollbackResult,
    PortraitNamedPromptProfileRows, PortraitPromptProfileRows,
};

#[cfg(test)]
pub(super) use rollback::generated_image_matches_any_portrait;
#[cfg(test)]
use rollback::generated_image_matches_portrait;
pub(super) use rollback::generated_image_metadata_matches_any_portrait;

#[cfg(test)]
use a4n_db_models::models::{ai::generated_image, character::character_portrait};
#[cfg(test)]
use serde_json::Value;

#[cfg(test)]
mod tests;
