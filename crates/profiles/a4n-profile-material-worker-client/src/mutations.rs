use super::*;

#[async_trait::async_trait]
impl ProfileMaterialMutationApplicationPort for ProfileMaterialWorkerClient {
    async fn set_primary(
        &self,
        user_id: i64,
        input: SetProfilePrimaryMaterialInput,
    ) -> Result<ProfileMaterialGroup, AppError> {
        let project_id = input.project_id;
        let response: ProfileMaterialSetPrimaryResponse = self
            .command(
                user_id,
                project_id,
                PROFILE_MATERIAL_SET_PRIMARY_OPERATION,
                input,
            )
            .await?;
        Ok(response.group)
    }
}
