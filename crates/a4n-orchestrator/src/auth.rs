pub use a4n_db::auth::BootstrapAdminInput;

use a4n_core::error::AppError;
use a4n_core::models::user::{invite_code, user, user_session};

use crate::OrchestratorContext;

impl OrchestratorContext {
    pub async fn auth_user_by_id(&self, user_id: i64) -> Result<user::Model, AppError> {
        a4n_db::auth::user_by_id(self.db.connection(), user_id).await
    }

    pub async fn current_user(&self, user_id: i64) -> Result<user::Model, AppError> {
        a4n_db::auth::current_user(self.db.connection(), user_id).await
    }

    pub async fn require_existing_user_or_bootstrap_admin(
        &self,
        bootstrap: Option<BootstrapAdminInput<'_>>,
    ) -> Result<(), AppError> {
        a4n_db::auth::require_existing_user_or_bootstrap_admin(self.db.connection(), bootstrap)
            .await
    }

    pub async fn register_user_with_invite(
        &self,
        username: &str,
        email: &str,
        password: &str,
        invite_code: &str,
    ) -> Result<user::Model, AppError> {
        a4n_db::auth::register_user_with_invite(
            self.db.connection(),
            username,
            email,
            password,
            invite_code,
        )
        .await
    }

    pub async fn login_user(
        &self,
        username: &str,
        password: &str,
    ) -> Result<user::Model, AppError> {
        a4n_db::auth::login_user(self.db.connection(), username, password).await
    }

    pub async fn create_session(
        &self,
        user_id: i64,
        refresh_token: &str,
        user_agent: Option<&str>,
    ) -> Result<user_session::Model, AppError> {
        a4n_db::auth::create_session(self.db.connection(), user_id, refresh_token, user_agent).await
    }

    pub async fn update_user_profile(
        &self,
        user_id: i64,
        email: Option<&str>,
        avatar_url: Option<Option<&str>>,
    ) -> Result<user::Model, AppError> {
        a4n_db::auth::update_profile(self.db.connection(), user_id, email, avatar_url).await
    }

    pub async fn change_user_password(
        &self,
        user_id: i64,
        old_password: &str,
        new_password: &str,
    ) -> Result<(), AppError> {
        a4n_db::auth::change_password(self.db.connection(), user_id, old_password, new_password)
            .await
    }

    pub async fn revoke_user_session(
        &self,
        user_id: i64,
        session_id: &str,
    ) -> Result<(), AppError> {
        a4n_db::auth::revoke_session(self.db.connection(), user_id, session_id).await
    }

    pub async fn verify_session_refresh_token(
        &self,
        user_id: i64,
        session_id: &str,
        refresh_token: &str,
    ) -> Result<user_session::Model, AppError> {
        a4n_db::auth::verify_session_refresh_token(
            self.db.connection(),
            user_id,
            session_id,
            refresh_token,
        )
        .await
    }

    pub async fn rotate_session_refresh_token(
        &self,
        session: user_session::Model,
        old_refresh_token: &str,
        new_refresh_token: &str,
    ) -> Result<user_session::Model, AppError> {
        a4n_db::auth::rotate_session_refresh_token(
            self.db.connection(),
            session,
            old_refresh_token,
            new_refresh_token,
        )
        .await
    }

    pub async fn user_sessions(&self, user_id: i64) -> Result<Vec<user_session::Model>, AppError> {
        a4n_db::auth::list_sessions(self.db.connection(), user_id).await
    }

    pub async fn create_auth_user(
        &self,
        username: &str,
        email: &str,
        password: &str,
        role: &str,
    ) -> Result<user::Model, AppError> {
        a4n_db::auth::create_user(self.db.connection(), username, email, password, role).await
    }

    pub async fn create_invite_code_record(
        &self,
        code: &str,
        created_by_user_id: Option<i64>,
    ) -> Result<invite_code::Model, AppError> {
        a4n_db::auth::create_invite_code(self.db.connection(), code, created_by_user_id).await
    }

    pub async fn update_auth_user_role(
        &self,
        user_id: i64,
        role: &str,
    ) -> Result<user::Model, AppError> {
        a4n_db::auth::update_user_role(self.db.connection(), user_id, role).await
    }

    pub async fn disable_auth_user(
        &self,
        caller_user_id: i64,
        user_id: i64,
    ) -> Result<(), AppError> {
        a4n_db::auth::disable_user(self.db.connection(), caller_user_id, user_id).await
    }

    pub async fn login_with_oauth2(
        &self,
        provider: &str,
        provider_id: &str,
        email: Option<&str>,
        username_hint: Option<&str>,
        avatar_url: Option<&str>,
    ) -> Result<user::Model, AppError> {
        a4n_db::auth::login_with_oauth2(
            self.db.connection(),
            provider,
            provider_id,
            email,
            username_hint,
            avatar_url,
        )
        .await
    }
}
