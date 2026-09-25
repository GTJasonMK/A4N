use a4n_db::{DbHandle, auth::BootstrapAdminInput};
use a4n_db_models::models::user::{invite_code, user, user_session};
use a4n_domain::auth::UserSummary;
use a4n_error::AppError;

use super::OrchestratorContext;
use crate::auth::{required_session_id, required_session_user_id};

#[derive(Clone)]
pub(super) struct AuthApplication {
    database: DbHandle,
}

impl AuthApplication {
    pub(super) fn new(database: DbHandle) -> Self {
        Self { database }
    }

    async fn user_by_id(&self, user_id: i64) -> Result<user::Model, AppError> {
        a4n_db::auth::user_by_id(self.database.connection(), user_id).await
    }

    async fn current_user_summary(&self, user_id: i64) -> Result<UserSummary, AppError> {
        a4n_db::auth::current_user_summary(self.database.connection(), user_id).await
    }

    async fn require_existing_user_or_bootstrap_admin(
        &self,
        bootstrap: Option<BootstrapAdminInput<'_>>,
    ) -> Result<(), AppError> {
        a4n_db::auth::require_existing_user_or_bootstrap_admin(
            self.database.connection(),
            bootstrap,
        )
        .await
    }

    async fn register_user_with_invite(
        &self,
        username: &str,
        email: &str,
        password: &str,
        invite_code: &str,
    ) -> Result<user::Model, AppError> {
        a4n_db::auth::register_user_with_invite(
            self.database.connection(),
            username,
            email,
            password,
            invite_code,
        )
        .await
    }

    async fn login_user(&self, username: &str, password: &str) -> Result<user::Model, AppError> {
        a4n_db::auth::login_user(self.database.connection(), username, password).await
    }

    async fn create_session(
        &self,
        user_id: i64,
        refresh_token: &str,
        user_agent: Option<&str>,
    ) -> Result<user_session::Model, AppError> {
        a4n_db::auth::create_session(
            self.database.connection(),
            user_id,
            refresh_token,
            user_agent,
        )
        .await
    }

    async fn update_user_profile(
        &self,
        user_id: i64,
        email: Option<&str>,
        avatar_url: Option<Option<&str>>,
        page_cli_enabled: Option<bool>,
    ) -> Result<user::Model, AppError> {
        a4n_db::auth::update_profile(
            self.database.connection(),
            user_id,
            email,
            avatar_url,
            page_cli_enabled,
        )
        .await
    }

    async fn change_user_password(
        &self,
        user_id: i64,
        old_password: &str,
        new_password: &str,
    ) -> Result<(), AppError> {
        a4n_db::auth::change_password(
            self.database.connection(),
            user_id,
            old_password,
            new_password,
        )
        .await
    }

    async fn revoke_user_session(&self, user_id: i64, session_id: &str) -> Result<(), AppError> {
        a4n_db::auth::revoke_session(self.database.connection(), user_id, session_id).await
    }

    async fn list_sessions(&self, user_id: i64) -> Result<Vec<user_session::Model>, AppError> {
        let user_id = required_session_user_id(user_id, "列出会话")?;
        a4n_db::auth::list_sessions(self.database.connection(), user_id).await
    }

    async fn revoke_session(&self, user_id: i64, session_id: &str) -> Result<(), AppError> {
        let user_id = required_session_user_id(user_id, "撤销会话")?;
        let session_id = required_session_id(session_id, "撤销会话")?;
        self.revoke_user_session(user_id, &session_id).await
    }

    async fn verify_session_refresh_token(
        &self,
        user_id: i64,
        session_id: &str,
        refresh_token: &str,
    ) -> Result<user_session::Model, AppError> {
        a4n_db::auth::verify_session_refresh_token(
            self.database.connection(),
            user_id,
            session_id,
            refresh_token,
        )
        .await
    }

    async fn verify_active_session(
        &self,
        user_id: i64,
        session_id: &str,
    ) -> Result<user_session::Model, AppError> {
        a4n_db::auth::verify_active_session(self.database.connection(), user_id, session_id).await
    }

    async fn rotate_session_refresh_token(
        &self,
        session: user_session::Model,
        old_refresh_token: &str,
        new_refresh_token: &str,
    ) -> Result<user_session::Model, AppError> {
        a4n_db::auth::rotate_session_refresh_token(
            self.database.connection(),
            session,
            old_refresh_token,
            new_refresh_token,
        )
        .await
    }

    async fn create_auth_user(
        &self,
        username: &str,
        email: &str,
        password: &str,
        role: &str,
    ) -> Result<user::Model, AppError> {
        a4n_db::auth::create_user(self.database.connection(), username, email, password, role).await
    }

    async fn create_invite_code_record(
        &self,
        code: &str,
        created_by_user_id: Option<i64>,
    ) -> Result<invite_code::Model, AppError> {
        a4n_db::auth::create_invite_code(self.database.connection(), code, created_by_user_id).await
    }

    async fn list_auth_users(&self) -> Result<Vec<UserSummary>, AppError> {
        a4n_db::auth::list_users(self.database.connection()).await
    }

    async fn list_invite_code_records(&self) -> Result<Vec<invite_code::Model>, AppError> {
        a4n_db::auth::list_invite_codes(self.database.connection()).await
    }

    async fn update_auth_user_role(
        &self,
        user_id: i64,
        role: &str,
    ) -> Result<user::Model, AppError> {
        a4n_db::auth::update_user_role(self.database.connection(), user_id, role).await
    }

    async fn login_with_oauth2(
        &self,
        provider: &str,
        provider_id: &str,
        email: Option<&str>,
        username_hint: Option<&str>,
        avatar_url: Option<&str>,
    ) -> Result<user::Model, AppError> {
        a4n_db::auth::login_with_oauth2(
            self.database.connection(),
            provider,
            provider_id,
            email,
            username_hint,
            avatar_url,
        )
        .await
    }
}

impl OrchestratorContext {
    pub(crate) async fn auth_user_by_id(&self, user_id: i64) -> Result<user::Model, AppError> {
        self.auth.user_by_id(user_id).await
    }

    pub(crate) async fn create_auth_user(
        &self,
        username: &str,
        email: &str,
        password: &str,
        role: &str,
    ) -> Result<user::Model, AppError> {
        self.auth
            .create_auth_user(username, email, password, role)
            .await
    }

    pub(crate) async fn create_invite_code_record(
        &self,
        code: &str,
        created_by_user_id: Option<i64>,
    ) -> Result<invite_code::Model, AppError> {
        self.auth
            .create_invite_code_record(code, created_by_user_id)
            .await
    }

    pub(crate) async fn list_auth_users(&self) -> Result<Vec<UserSummary>, AppError> {
        self.auth.list_auth_users().await
    }

    pub(crate) async fn list_invite_code_records(
        &self,
    ) -> Result<Vec<invite_code::Model>, AppError> {
        self.auth.list_invite_code_records().await
    }

    pub(crate) async fn update_auth_user_role(
        &self,
        user_id: i64,
        role: &str,
    ) -> Result<user::Model, AppError> {
        self.auth.update_auth_user_role(user_id, role).await
    }
}

#[async_trait::async_trait]
impl a4n_api_contract::auth::AuthReadApplicationPort for OrchestratorContext {
    async fn current_user(
        &self,
        user_id: i64,
    ) -> Result<a4n_api_contract::auth::UserSummary, AppError> {
        self.auth.current_user_summary(user_id).await
    }

    async fn list_sessions(
        &self,
        user_id: i64,
    ) -> Result<Vec<a4n_api_contract::auth::SessionSummary>, AppError> {
        let sessions = self.auth.list_sessions(user_id).await?;
        Ok(sessions.iter().map(auth_session_summary).collect())
    }

    async fn verify_session_refresh_token(
        &self,
        user_id: i64,
        session_id: &str,
        refresh_token: &str,
    ) -> Result<(), AppError> {
        self.auth
            .verify_session_refresh_token(user_id, session_id, refresh_token)
            .await
            .map(|_| ())
    }

    async fn verify_active_session(&self, user_id: i64, session_id: &str) -> Result<(), AppError> {
        self.auth
            .verify_active_session(user_id, session_id)
            .await
            .map(|_| ())
    }
}

#[async_trait::async_trait]
impl a4n_api_contract::auth::AuthMutationApplicationPort for OrchestratorContext {
    async fn require_existing_user_or_bootstrap_admin(
        &self,
        bootstrap: Option<(&str, &str, &str)>,
    ) -> Result<(), AppError> {
        let bootstrap = bootstrap.map(|(username, email, password)| BootstrapAdminInput {
            username,
            email,
            password,
        });
        self.auth
            .require_existing_user_or_bootstrap_admin(bootstrap)
            .await
    }

    async fn register_user(
        &self,
        input: a4n_api_contract::auth::RegisterInput,
    ) -> Result<a4n_api_contract::auth::UserSummary, AppError> {
        let user = self
            .auth
            .register_user_with_invite(
                &input.username,
                &input.email,
                &input.password,
                &input.invite_code,
            )
            .await?;
        Ok(auth_user_summary(&user))
    }

    async fn login_user(
        &self,
        input: a4n_api_contract::auth::LoginInput,
    ) -> Result<a4n_api_contract::auth::UserSummary, AppError> {
        let user = self
            .auth
            .login_user(&input.username, &input.password)
            .await?;
        Ok(auth_user_summary(&user))
    }

    async fn create_session(
        &self,
        user_id: i64,
        refresh_token: &str,
        user_agent: Option<&str>,
    ) -> Result<a4n_api_contract::auth::SessionSummary, AppError> {
        let session = self
            .auth
            .create_session(user_id, refresh_token, user_agent)
            .await?;
        Ok(auth_session_summary(&session))
    }

    async fn revoke_session(&self, user_id: i64, session_id: &str) -> Result<(), AppError> {
        self.auth.revoke_session(user_id, session_id).await
    }

    async fn update_profile(
        &self,
        user_id: i64,
        input: a4n_api_contract::auth::UpdateProfileInput,
    ) -> Result<a4n_api_contract::auth::UserSummary, AppError> {
        let avatar_url = input.avatar_url.as_ref().map(|value| value.as_deref());
        let user = self
            .auth
            .update_user_profile(
                user_id,
                input.email.as_deref(),
                avatar_url,
                input.page_cli_enabled,
            )
            .await?;
        Ok(auth_user_summary(&user))
    }

    async fn change_password(
        &self,
        user_id: i64,
        input: a4n_api_contract::auth::ChangePasswordInput,
    ) -> Result<(), AppError> {
        self.auth
            .change_user_password(user_id, &input.old_password, &input.new_password)
            .await
    }

    async fn rotate_session_refresh_token(
        &self,
        user_id: i64,
        session_id: &str,
        old_refresh_token: &str,
        new_refresh_token: &str,
    ) -> Result<(), AppError> {
        let user_id = required_session_user_id(user_id, "刷新令牌")?;
        let session_id = required_session_id(session_id, "刷新令牌")?;
        let session = self
            .auth
            .verify_session_refresh_token(user_id, &session_id, old_refresh_token)
            .await?;
        self.auth
            .rotate_session_refresh_token(session, old_refresh_token, new_refresh_token)
            .await
            .map(|_| ())
    }

    async fn login_with_oauth2(
        &self,
        provider: &str,
        provider_id: &str,
        email: Option<&str>,
        username_hint: Option<&str>,
        avatar_url: Option<&str>,
    ) -> Result<a4n_api_contract::auth::UserSummary, AppError> {
        let user = self
            .auth
            .login_with_oauth2(provider, provider_id, email, username_hint, avatar_url)
            .await?;
        Ok(auth_user_summary(&user))
    }
}

pub(super) fn auth_user_summary(user: &user::Model) -> a4n_api_contract::auth::UserSummary {
    a4n_api_contract::auth::UserSummary {
        id: user.id,
        username: user.username.clone(),
        email: user.email.clone(),
        role: user.role.clone(),
        oauth_provider: user.oauth_provider.clone(),
        oauth_subject: user.oauth_subject.clone(),
        avatar_url: user.avatar_url.clone(),
        page_cli_enabled: user.page_cli_enabled,
        is_active: user.is_active,
        created_at: Some(user.created_at.and_utc().to_rfc3339()),
        updated_at: Some(user.updated_at.and_utc().to_rfc3339()),
        last_login_at: user.last_login_at.map(|value| value.and_utc().to_rfc3339()),
    }
}

fn auth_session_summary(session: &user_session::Model) -> a4n_api_contract::auth::SessionSummary {
    a4n_api_contract::auth::SessionSummary {
        id: session.id.clone(),
        user_agent: session.user_agent.clone(),
        created_at: session.created_at.and_utc().to_rfc3339(),
        expires_at: session.expires_at.and_utc().to_rfc3339(),
        last_used_at: session
            .last_used_at
            .map(|value| value.and_utc().to_rfc3339()),
    }
}
