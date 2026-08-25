mod conflicts;
mod mutations;
mod projections;
mod queries;

pub use a4n_api_contract::profile::{
    CreateObjectProfileInput, DeleteObjectProfileInput, ListObjectProfilesInput,
    ObjectProfileDeleteResult, ObjectProfileMutationResult, ObjectProfileSummary,
    UpdateObjectProfileInput,
};
pub use mutations::{create_object_profile, delete_object_profile, update_object_profile};
pub use queries::list_object_profiles;

#[cfg(test)]
mod tests;
