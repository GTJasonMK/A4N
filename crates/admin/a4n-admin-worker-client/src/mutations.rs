use super::*;

#[async_trait::async_trait]
impl AdminMutationApplicationPort for AdminWorkerClient {
    async fn create_user(
        &self,
        caller_user_id: i64,
        input: CreateUserInput,
    ) -> Result<UserSummary, AppError> {
        self.command(
            caller_user_id,
            ADMIN_CREATE_USER_OPERATION,
            AdminCreateUserInput {
                caller_user_id,
                input,
            },
        )
        .await
    }

    async fn create_invite_code(
        &self,
        caller_user_id: i64,
        input: CreateInviteCodeInput,
    ) -> Result<InviteCodeSummary, AppError> {
        self.command(
            caller_user_id,
            ADMIN_CREATE_INVITE_OPERATION,
            AdminCreateInviteInput {
                caller_user_id,
                input,
            },
        )
        .await
    }

    async fn update_user_role(
        &self,
        caller_user_id: i64,
        user_id: i64,
        input: UpdateUserRoleInput,
    ) -> Result<UserSummary, AppError> {
        self.command(
            caller_user_id,
            ADMIN_UPDATE_ROLE_OPERATION,
            AdminUpdateRoleInput {
                caller_user_id,
                user_id,
                input,
            },
        )
        .await
    }
}
