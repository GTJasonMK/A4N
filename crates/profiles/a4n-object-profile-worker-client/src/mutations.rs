use super::*;

#[async_trait::async_trait]
impl ObjectProfileMutationApplicationPort for ObjectProfileWorkerClient {
    async fn create(
        &self,
        user_id: i64,
        input: CreateObjectProfileInput,
    ) -> Result<ObjectProfileMutationOutcome, AppError> {
        let response: ObjectProfileMutationResponse = self
            .command(
                user_id,
                Some(input.project_id),
                OBJECT_PROFILE_CREATE_OPERATION,
                input,
            )
            .await?;
        Ok(ObjectProfileMutationOutcome {
            result: response.result,
            downstream_image_ids: response.downstream_image_ids,
        })
    }

    async fn update(
        &self,
        user_id: i64,
        input: UpdateObjectProfileInput,
    ) -> Result<ObjectProfileMutationOutcome, AppError> {
        let response: ObjectProfileMutationResponse = self
            .command(user_id, None, OBJECT_PROFILE_UPDATE_OPERATION, input)
            .await?;
        Ok(ObjectProfileMutationOutcome {
            result: response.result,
            downstream_image_ids: response.downstream_image_ids,
        })
    }

    async fn delete(
        &self,
        user_id: i64,
        input: DeleteObjectProfileInput,
    ) -> Result<ObjectProfileDeleteOutcome, AppError> {
        let response: ObjectProfileDeleteResponse = self
            .command(user_id, None, OBJECT_PROFILE_DELETE_OPERATION, input)
            .await?;
        Ok(ObjectProfileDeleteOutcome {
            result: response.result,
            downstream_image_ids: response.downstream_image_ids,
        })
    }
}
