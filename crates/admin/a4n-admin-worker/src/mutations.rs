use super::*;

impl WorkerState {
    pub(super) async fn create_user(
        &self,
        request: WorkerEnvelope<AdminCreateUserInput>,
    ) -> Result<a4n_admin_worker_contract::UserSummary, WorkerFailure> {
        validate_request(&request, ADMIN_CREATE_USER_OPERATION)?;
        let caller = self.caller(&request.principal, request.payload.caller_user_id)?;
        self.require_admin(caller, AdminPermission::ManageUsers)
            .await?;
        let input = request.payload.input;
        a4n_db::auth::create_user_summary(
            self.database.connection(),
            &input.username,
            &input.email,
            &input.password,
            &input.role,
        )
        .await
        .map_err(app_error)
    }

    pub(super) async fn create_invite(
        &self,
        request: WorkerEnvelope<AdminCreateInviteInput>,
    ) -> Result<a4n_admin_worker_contract::InviteCodeSummary, WorkerFailure> {
        validate_request(&request, ADMIN_CREATE_INVITE_OPERATION)?;
        let caller = self.caller(&request.principal, request.payload.caller_user_id)?;
        self.require_admin(caller, AdminPermission::ManageUsers)
            .await?;
        a4n_db::auth::create_invite_code(
            self.database.connection(),
            &request.payload.input.code,
            Some(caller),
        )
        .await
        .map_err(app_error)
    }

    pub(super) async fn update_role(
        &self,
        request: WorkerEnvelope<AdminUpdateRoleInput>,
    ) -> Result<a4n_admin_worker_contract::UserSummary, WorkerFailure> {
        validate_request(&request, ADMIN_UPDATE_ROLE_OPERATION)?;
        let caller = self.caller(&request.principal, request.payload.caller_user_id)?;
        self.require_admin(caller, AdminPermission::ManageUsers)
            .await?;
        a4n_db::auth::update_user_role_summary(
            self.database.connection(),
            request.payload.user_id,
            &request.payload.input.role,
        )
        .await
        .map_err(app_error)
    }
}
