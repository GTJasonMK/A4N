use super::*;

#[async_trait::async_trait]
impl ProjectStatusMutationApplicationPort for ProjectStatusWorkerClient {
    async fn forward_status(
        &self,
        user_id: i64,
        project_id: i64,
        target: ProjectStatus,
    ) -> Result<ProjectStatusMutationResult, AppError> {
        self.command(
            user_id,
            project_id,
            PROJECT_STATUS_FORWARD_OPERATION,
            ProjectStatusForwardInput { project_id, target },
        )
        .await
    }

    async fn rollback_status(
        &self,
        user_id: i64,
        project_id: i64,
        expected_from: ProjectStatus,
        target: ProjectStatus,
    ) -> Result<ProjectStatusMutationResult, AppError> {
        self.command(
            user_id,
            project_id,
            PROJECT_STATUS_ROLLBACK_OPERATION,
            ProjectStatusRollbackInput {
                project_id,
                expected_from,
                target,
            },
        )
        .await
    }
}
