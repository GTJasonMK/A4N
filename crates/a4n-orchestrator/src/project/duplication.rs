use a4n_core::error::AppError;
use async_trait::async_trait;

use super::{DerivedProjectMediaFileCopier, FreeformProjectMediaFileCopier};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DuplicateProjectApplicationInput {
    pub user_id: i64,
    pub project_id: i64,
    pub new_title: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BranchProjectApplicationInput {
    pub user_id: i64,
    pub project_id: i64,
    pub branch_title: String,
    pub at_status: String,
}

#[async_trait]
pub trait ProjectDuplicationHostPort:
    DerivedProjectMediaFileCopier + FreeformProjectMediaFileCopier + Send
{
    async fn verify_project_editor(&self, user_id: i64, project_id: i64) -> Result<(), AppError>;

    async fn rollback_copied_media_files(&mut self, label: &str) -> Result<(), String>;
}

#[cfg(test)]
mod tests;
