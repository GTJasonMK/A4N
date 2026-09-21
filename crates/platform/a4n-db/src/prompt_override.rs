mod cleanup;
mod hydration;
mod identity;
mod storage;
mod types;

pub use cleanup::{
    delete_project_overrides_for_prompt_names, preflight_project_override_keys_for_retired_cleanup,
};
pub use hydration::{project_override_hydration_plan, project_override_summaries};
pub use identity::{normalize_project_override_identity, validate_project_override_content};
pub use storage::{
    delete_project_override, load_project_override_persisted_content, save_project_override,
};
pub use types::{
    ProjectOverrideHydration, ProjectOverrideIdentity, ProjectPromptOverrideSummary,
    PromptOverrideDbError,
};

#[cfg(test)]
use identity::parse_project_override_key;

#[cfg(test)]
mod tests;
