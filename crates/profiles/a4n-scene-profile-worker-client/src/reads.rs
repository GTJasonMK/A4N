use super::*;

#[async_trait::async_trait]
impl SceneProfileReadApplicationPort for SceneProfileWorkerClient {
    async fn list(
        &self,
        user_id: i64,
        project_id: i64,
    ) -> Result<Vec<SceneProfileSummary>, AppError> {
        self.command(
            user_id,
            Some(project_id),
            SCENE_PROFILE_LIST_OPERATION,
            SceneProfileProjectInput { project_id },
        )
        .await
    }
}
