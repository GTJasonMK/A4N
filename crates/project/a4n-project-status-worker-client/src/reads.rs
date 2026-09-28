use super::*;

#[async_trait::async_trait]
impl ProjectStatusReadApplicationPort for ProjectStatusWorkerClient {
    async fn status_machine(
        &self,
        user_id: i64,
        project_id: i64,
    ) -> Result<ProjectStatusMachineSummary, AppError> {
        self.command(
            user_id,
            project_id,
            PROJECT_STATUS_MACHINE_OPERATION,
            ProjectStatusProjectInput { project_id },
        )
        .await
    }

    async fn preview_rollback(
        &self,
        user_id: i64,
        project_id: i64,
        target: ProjectStatus,
    ) -> Result<ProjectRollbackPreview, AppError> {
        self.command(
            user_id,
            project_id,
            PROJECT_STATUS_PREVIEW_ROLLBACK_OPERATION,
            ProjectStatusRollbackInput { project_id, target },
        )
        .await
    }
}
