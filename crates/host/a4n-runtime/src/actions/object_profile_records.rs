mod mutations;
mod queries;

pub use a4n_api_contract::profile::{
    CreateObjectProfileInput, DeleteObjectProfileInput, ListObjectProfilesInput,
    ObjectProfileDeleteResult, ObjectProfileListItem, ObjectProfileMutationResult,
    ObjectProfileSummary, UpdateObjectProfileInput,
};
pub use mutations::{create_object_profile, delete_object_profile, update_object_profile};
pub use queries::{list_object_profile_index, list_object_profiles};

#[cfg(test)]
mod tests;
