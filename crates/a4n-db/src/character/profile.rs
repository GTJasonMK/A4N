mod creation;
mod deletion;
mod records;
mod state;
mod types;

pub use creation::{create_profile, create_profile_record, create_profile_with_generation_run};
pub(crate) use deletion::delete_profile_cascade_in;
pub use deletion::delete_profile_cascade_transaction;
pub(in crate::character) use records::require_profile;
pub use records::{
    profile_by_id, profile_by_project_name, profiles_by_project, update_profile_current_state,
};
pub use state::{
    apply_profile_current_state, normalize_base_attributes, profile_current_state_fields,
};
pub use types::{
    CharacterProfileCreateError, CharacterProfileCreateInput, CharacterProfileCurrentStateFields,
    CharacterProfileCurrentStateUpdate, CharacterProfileDeleteError,
    CharacterProfileDeletionSummary,
};

#[cfg(test)]
mod tests;
