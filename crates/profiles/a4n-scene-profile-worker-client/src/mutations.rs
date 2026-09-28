use super::*;

#[async_trait::async_trait]
impl SceneProfileMutationApplicationPort for SceneProfileWorkerClient {
    async fn create(
        &self,
        user_id: i64,
        input: CreateSceneProfileInput,
    ) -> Result<SceneProfileMutationOutcome, AppError> {
        let project_id = input.project_id;
        let response: SceneProfileMutationWorkerResult = self
            .command(
                user_id,
                Some(project_id),
                SCENE_PROFILE_CREATE_OPERATION,
                input,
            )
            .await?;
        Ok(SceneProfileMutationOutcome {
            result: response.result,
            downstream_image_ids: response.downstream_image_ids,
        })
    }

    async fn update(
        &self,
        user_id: i64,
        input: UpdateSceneProfileInput,
    ) -> Result<SceneProfileMutationOutcome, AppError> {
        let response: SceneProfileMutationWorkerResult = self
            .command(user_id, None, SCENE_PROFILE_UPDATE_OPERATION, input)
            .await?;
        Ok(SceneProfileMutationOutcome {
            result: response.result,
            downstream_image_ids: response.downstream_image_ids,
        })
    }

    async fn delete(
        &self,
        user_id: i64,
        input: DeleteSceneProfileInput,
    ) -> Result<SceneProfileDeleteOutcome, AppError> {
        let response: SceneProfileDeleteWorkerResult = self
            .command(user_id, None, SCENE_PROFILE_DELETE_OPERATION, input)
            .await?;
        Ok(SceneProfileDeleteOutcome {
            result: response.result,
            project_id: response.project_id,
            downstream_image_ids: response.downstream_image_ids,
        })
    }
}
