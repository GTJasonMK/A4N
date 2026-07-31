use a4n_core::{
    error::AppError,
    models::user::{user, user_session},
};
use a4n_orchestrator::{OrchestratorContext, auth::BootstrapAdminInput};

pub(crate) struct AuthRuntimeFacade<'a> {
    orchestrator: &'a OrchestratorContext,
}

impl<'a> AuthRuntimeFacade<'a> {
    pub(super) fn new(orchestrator: &'a OrchestratorContext) -> Self {
        Self { orchestrator }
    }

    pub(crate) async fn user_by_id(&self, user_id: i64) -> Result<user::Model, AppError> {
        self.orchestrator.auth_user_by_id(user_id).await
    }

    pub(crate) async fn current_user(&self, user_id: i64) -> Result<user::Model, AppError> {
        self.orchestrator.current_user(user_id).await
    }

    pub(crate) async fn require_existing_user_or_bootstrap_admin(
        &self,
        bootstrap: Option<BootstrapAdminInput<'_>>,
    ) -> Result<(), AppError> {
        self.orchestrator
            .require_existing_user_or_bootstrap_admin(bootstrap)
            .await
    }

    pub(crate) async fn register_user_with_invite(
        &self,
        username: &str,
        email: &str,
        password: &str,
        invite_code: &str,
    ) -> Result<user::Model, AppError> {
        self.orchestrator
            .register_user_with_invite(username, email, password, invite_code)
            .await
    }

    pub(crate) async fn login_user(
        &self,
        username: &str,
        password: &str,
    ) -> Result<user::Model, AppError> {
        self.orchestrator.login_user(username, password).await
    }

    pub(crate) async fn create_session(
        &self,
        user_id: i64,
        refresh_token: &str,
        user_agent: Option<&str>,
    ) -> Result<user_session::Model, AppError> {
        self.orchestrator
            .create_session(user_id, refresh_token, user_agent)
            .await
    }

    pub(crate) async fn update_own_user_profile(
        &self,
        caller_user_id: i64,
        target_user_id: i64,
        email: Option<&Option<String>>,
        avatar_url: Option<Option<&str>>,
    ) -> Result<user::Model, AppError> {
        self.orchestrator
            .update_own_user_profile(caller_user_id, target_user_id, email, avatar_url)
            .await
    }

    pub(crate) async fn change_own_user_password(
        &self,
        caller_user_id: i64,
        target_user_id: i64,
        old_password: &str,
        new_password: &str,
    ) -> Result<(), AppError> {
        self.orchestrator
            .change_own_user_password(caller_user_id, target_user_id, old_password, new_password)
            .await
    }

    pub(crate) async fn logout_user_session(
        &self,
        user_id: Option<i64>,
        session_id: Option<&str>,
    ) -> Result<(), AppError> {
        self.orchestrator
            .logout_user_session(user_id, session_id)
            .await
    }

    pub(crate) async fn list_own_sessions(
        &self,
        caller_user_id: i64,
        target_user_id: i64,
    ) -> Result<Vec<user_session::Model>, AppError> {
        self.orchestrator
            .list_own_sessions(caller_user_id, target_user_id)
            .await
    }

    pub(crate) async fn revoke_own_session(
        &self,
        caller_user_id: i64,
        target_user_id: i64,
        session_id: &str,
    ) -> Result<(), AppError> {
        self.orchestrator
            .revoke_own_session(caller_user_id, target_user_id, session_id)
            .await
    }

    pub(crate) async fn verify_session_refresh_token(
        &self,
        user_id: i64,
        session_id: &str,
        refresh_token: &str,
    ) -> Result<user_session::Model, AppError> {
        self.orchestrator
            .verify_session_refresh_token(user_id, session_id, refresh_token)
            .await
    }

    pub(crate) async fn verify_active_session(
        &self,
        user_id: i64,
        session_id: &str,
    ) -> Result<user_session::Model, AppError> {
        self.orchestrator
            .verify_active_session(user_id, session_id)
            .await
    }

    pub(crate) async fn rotate_session_refresh_token(
        &self,
        session: user_session::Model,
        old_refresh_token: &str,
        new_refresh_token: &str,
    ) -> Result<user_session::Model, AppError> {
        self.orchestrator
            .rotate_session_refresh_token(session, old_refresh_token, new_refresh_token)
            .await
    }

    pub(crate) async fn login_with_oauth2(
        &self,
        provider: &str,
        provider_id: &str,
        email: Option<&str>,
        username_hint: Option<&str>,
        avatar_url: Option<&str>,
    ) -> Result<user::Model, AppError> {
        self.orchestrator
            .login_with_oauth2(provider, provider_id, email, username_hint, avatar_url)
            .await
    }

    pub(crate) fn required_jwt_secret(&self) -> Result<&'a str, AppError> {
        self.orchestrator.required_jwt_secret()
    }
}
