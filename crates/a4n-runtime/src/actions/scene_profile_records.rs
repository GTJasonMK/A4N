mod conflicts;
mod mutations;
mod projections;
mod queries;

pub use a4n_api_contract::profile::{
    CreateSceneProfileInput, DeleteSceneProfileInput, ListSceneProfilesInput,
    SceneProfileDeleteResult, SceneProfileMutationResult, SceneProfileSummary,
    UpdateSceneProfileInput,
};
pub use mutations::{create_scene_profile, delete_scene_profile, update_scene_profile};
pub use queries::list_scene_profiles;

#[cfg(test)]
mod tests;
