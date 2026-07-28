#[cfg(test)]
mod tests;

mod execution;
mod media;

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct DuplicateProjectInput {
    pub project_id: i64,
    pub new_title: String,
}

pub use execution::{branch_project, duplicate_project};

#[cfg(test)]
use media::RuntimeProjectMediaFileCopier;
