mod deletion;
mod execution;
mod persistence;

pub(crate) use deletion::{GalgameScriptCleanupResult, delete_galgame_script_records_for_project};
pub use execution::replace_galgame_script_for_project_record;
#[cfg(test)]
use persistence::persist_galgame_script_record;

#[cfg(test)]
mod tests;
