mod character_discovery_flow;
mod contracts;
mod preview;
mod progress;
mod scope;
#[cfg(test)]
mod state_projection;
mod sync_batch;
mod sync_profile;
mod sync_trace;
mod sync_types;

pub(crate) use character_discovery_flow::character_discovery_preview_steps;
pub use character_discovery_flow::discover_character_profiles_for_chapters;
pub use contracts::{CharacterAiPreviewStep, CharacterAiTaskScope};
pub use preview::{
    preview_discover_character_profiles_request, preview_sync_all_character_profiles_request,
    preview_sync_character_from_chapters_request,
};
pub use progress::{CharacterAiProgress, CharacterAiProgressReporter};
pub use scope::{
    character_discovery_task_scope, character_profile_sync_all_task_scope,
    character_profile_sync_task_scope,
};
pub(crate) use sync_batch::sync_all_character_profiles_preview_steps;
pub use sync_batch::sync_all_character_profiles_to_chapter;
pub(crate) use sync_profile::character_sync_preview_steps;
pub use sync_profile::sync_character_profile_from_chapters;
pub(crate) use sync_trace::RuntimeCharacterProfileSyncStageTracePort;
pub use sync_types::CHARACTER_PROFILE_SYNC_STEPS;

#[cfg(test)]
mod tests;
