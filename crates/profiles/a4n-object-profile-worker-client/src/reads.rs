use super::*;

#[async_trait::async_trait]
impl ObjectProfileReadApplicationPort for ObjectProfileWorkerClient {
    async fn list(
        &self,
        user_id: i64,
        project_id: i64,
    ) -> Result<Vec<ObjectProfileSummary>, AppError> {
        self.command(
            user_id,
            Some(project_id),
            OBJECT_PROFILE_LIST_OPERATION,
            ObjectProfileProjectInput { project_id },
        )
        .await
    }

    async fn list_index(
        &self,
        user_id: i64,
        project_id: i64,
    ) -> Result<Vec<ObjectProfileListItem>, AppError> {
        self.command(
            user_id,
            Some(project_id),
            OBJECT_PROFILE_LIST_INDEX_OPERATION,
            ObjectProfileProjectInput { project_id },
        )
        .await
    }
}
