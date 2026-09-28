use super::*;

#[async_trait::async_trait]
impl ForeshadowingReadApplicationPort for ForeshadowingWorkerClient {
    async fn list(
        &self,
        user_id: i64,
        project_id: i64,
    ) -> Result<Vec<ForeshadowingSummary>, AppError> {
        self.command(
            user_id,
            project_id,
            FORESHADOWING_LIST_OPERATION,
            ForeshadowingProjectInput { project_id },
        )
        .await
    }
}
