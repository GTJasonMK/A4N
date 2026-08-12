mod conflicts;
mod draft;
mod mutations;
mod projections;
mod queries;

pub use a4n_api_contract::profile::{
    CreateObjectProfileInput, DeleteObjectProfileInput, ListObjectProfilesInput,
    ObjectProfileDeleteResult, ObjectProfileInput, ObjectProfileMutationResult,
    ObjectProfileSummary, UpdateObjectProfileInput,
};
pub use mutations::*;
pub use queries::list_object_profiles;

#[cfg(test)]
mod tests;
