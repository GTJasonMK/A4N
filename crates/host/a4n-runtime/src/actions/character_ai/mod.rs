mod character_discovery_flow;
mod contracts;
mod preview;
mod progress;
mod scope;
#[cfg(test)]
mod state_projection;
mod sync;

pub(crate) use character_discovery_flow::character_discovery_preview_step;
pub use character_discovery_flow::discover_character_profiles;
pub use contracts::{
    CharacterAiTaskScope, DiscoverCharacterProfilesInput, SyncAllCharacterProfilesInput,
    SyncAllCharacterProfilesResult, SyncCharacterProfileInput, SyncCharacterProfileResult,
};
pub use preview::{
    preview_discover_character_profiles_request, preview_sync_all_character_profiles_request,
    preview_sync_character_from_chapters_request,
};
pub use progress::{CharacterAiProgress, CharacterAiProgressReporter};
pub use scope::{
    character_discovery_task_scope, character_profile_sync_all_task_scope,
    character_profile_sync_task_scope,
};
pub(crate) use sync::batch::sync_all_character_profiles_preview_steps;
pub use sync::batch::sync_all_character_profiles_to_chapter;
pub(crate) use sync::profile::character_sync_preview_step;
pub use sync::profile::sync_character_profile_from_chapters;
pub(crate) use sync::trace::RuntimeCharacterProfileSyncStageTracePort;

#[cfg(test)]
mod tests;
