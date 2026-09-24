pub(super) use a4n_auth_application::AuthApplication;

use a4n_domain::admin::InviteCodeSummary;
use a4n_domain::auth::UserSummary;
use a4n_error::AppError;

use super::OrchestratorContext;

impl OrchestratorContext {
    pub fn auth_application(&self) -> AuthApplication {
        self.auth.clone()
    }

    pub(crate) async fn auth_user_by_id(&self, user_id: i64) -> Result<UserSummary, AppError> {
        self.auth.auth_user_by_id(user_id).await
    }

    pub(crate) async fn create_auth_user(
        &self,
        username: &str,
        email: &str,
        password: &str,
        role: &str,
    ) -> Result<UserSummary, AppError> {
        self.auth
            .create_auth_user(username, email, password, role)
            .await
    }

    pub(crate) async fn create_invite_code_record(
        &self,
        code: &str,
        created_by_user_id: Option<i64>,
    ) -> Result<InviteCodeSummary, AppError> {
        self.auth
            .create_invite_code_record(code, created_by_user_id)
            .await
    }

    pub(crate) async fn list_auth_users(&self) -> Result<Vec<UserSummary>, AppError> {
        self.auth.list_auth_users().await
    }

    pub(crate) async fn list_invite_code_records(
        &self,
    ) -> Result<Vec<InviteCodeSummary>, AppError> {
        self.auth.list_invite_code_records().await
    }

    pub(crate) async fn update_auth_user_role(
        &self,
        user_id: i64,
        role: &str,
    ) -> Result<UserSummary, AppError> {
        self.auth.update_auth_user_role(user_id, role).await
    }
}
