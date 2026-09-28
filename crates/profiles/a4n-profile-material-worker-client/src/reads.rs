use super::*;

#[async_trait::async_trait]
impl ProfileMaterialReadApplicationPort for ProfileMaterialWorkerClient {
    async fn list(
        &self,
        user_id: i64,
        input: ListProfileMaterialsInput,
    ) -> Result<Vec<ProfileMaterialGroup>, AppError> {
        let project_id = input.project_id;
        self.command(user_id, project_id, PROFILE_MATERIAL_LIST_OPERATION, input)
            .await
    }
}
