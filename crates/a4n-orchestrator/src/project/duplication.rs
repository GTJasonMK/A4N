use a4n_api_contract::project::ProjectSummary;
use a4n_domain::project::ProjectMediaFileCopier;
use a4n_error::AppError;
use async_trait::async_trait;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DuplicateProjectApplicationInput {
    pub user_id: i64,
    pub project_id: i64,
    pub new_title: String,
}

#[async_trait]
pub trait ProjectDuplicationHostPort: ProjectMediaFileCopier + Send + Sync {
    async fn verify_project_editor(&self, user_id: i64, project_id: i64) -> Result<(), AppError>;

    async fn rollback_copied_media_files(&mut self, label: &str) -> Result<(), String>;
}

#[async_trait]
pub trait ProjectDuplicationApplicationPort: Send + Sync {
    async fn duplicate_project(
        &self,
        input: DuplicateProjectApplicationInput,
        host_port: &mut dyn ProjectDuplicationHostPort,
    ) -> Result<ProjectSummary, AppError>;
}

#[cfg(test)]
mod tests;
