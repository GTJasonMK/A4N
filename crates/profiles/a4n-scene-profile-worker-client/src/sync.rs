use super::*;

impl SceneProfileWorkerClient {
    pub async fn replace_chapter_sync_records(
        &self,
        user_id: i64,
        input: SceneProfileSyncRecordsInput,
    ) -> Result<SceneSyncRecordResult, AppError> {
        self.command(
            user_id,
            Some(input.project_id),
            SCENE_PROFILE_SYNC_RECORDS_OPERATION,
            input,
        )
        .await
    }
}

#[async_trait::async_trait]
impl SceneProfileRecordSyncApplicationPort for SceneProfileWorkerClient {
    async fn replace_chapter_scene_sync_records(
        &self,
        user_id: i64,
        input: SceneProfileSyncRecordsInput,
    ) -> Result<(), AppError> {
        self.replace_chapter_sync_records(user_id, input)
            .await
            .map(|_| ())
    }
}
