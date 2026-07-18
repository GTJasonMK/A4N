mod artifacts;
mod history;
mod mutations;
mod readiness;
mod types;

pub use artifacts::project_status_artifacts;
pub use history::{
    delete_project_status_history, insert_project_status_history, project_status_history,
};
pub use mutations::update_project_status_record;
pub(crate) use mutations::update_project_status_record_in;
pub use readiness::chapter_generation_readiness;
pub use types::*;

#[cfg(test)]
use history::{normalize_optional_text, project_status_history_summary};
#[cfg(test)]
mod tests;
