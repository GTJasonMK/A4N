mod generation;
mod paths;
mod project;

pub(super) use generation::process_claimed_generation_cleanup;
pub(super) use project::{process_claimed_cleanup, process_claimed_file_artifact_cleanup};
