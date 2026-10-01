pub(super) use a4n_auth_application::AuthApplication;

use a4n_domain::auth::{UserAuthorizationRecord, UserSummary};
use a4n_error::AppError;

use super::OrchestratorContext;

impl OrchestratorContext {
    pub fn auth_application(&self) -> AuthApplication {
        self.auth.clone()
    }

    pub(crate) async fn auth_user_by_id(&self, user_id: i64) -> Result<UserSummary, AppError> {
        self.auth.auth_user_by_id(user_id).await
    }

    pub(crate) async fn auth_user_authorization_by_id(
        &self,
        user_id: i64,
    ) -> Result<UserAuthorizationRecord, AppError> {
        self.auth.auth_user_authorization_by_id(user_id).await
    }
}
