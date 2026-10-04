use a4n_api_contract::auth::{
    AuthMutationApplicationPort, AuthReadApplicationPort, ChangePasswordInput, LoginInput,
    RegisterInput, SessionSummary, UpdateProfileInput, UserSummary, required_session_id,
    required_session_user_id,
};
use a4n_db::{DbHandle, auth::BootstrapAdminInput};
use a4n_domain::admin::InviteCodeSummary;
use a4n_error::AppError;

#[derive(Clone)]
pub struct AuthApplication {
    database: DbHandle,
}

impl AuthApplication {
    pub fn new(database: DbHandle) -> Self {
        Self { database }
    }

    pub async fn auth_user_by_id(&self, user_id: i64) -> Result<UserSummary, AppError> {
        a4n_db::auth::user_summary_by_id(self.database.executor(), user_id).await
    }

    pub async fn auth_user_authorization_by_id(
        &self,
        user_id: i64,
    ) -> Result<a4n_domain::auth::UserAuthorizationRecord, AppError> {
        a4n_db::auth::user_authorization_by_id(self.database.executor(), user_id).await
    }

    pub async fn create_auth_user(
        &self,
        username: &str,
        email: &str,
        password: &str,
        role: &str,
    ) -> Result<UserSummary, AppError> {
        a4n_db::auth::create_user_summary(self.database.executor(), username, email, password, role)
            .await
    }

    pub async fn create_invite_code_record(
        &self,
        code: &str,
        created_by_user_id: Option<i64>,
    ) -> Result<InviteCodeSummary, AppError> {
        a4n_db::auth::create_invite_code(self.database.executor(), code, created_by_user_id).await
    }

    pub async fn list_auth_users(&self) -> Result<Vec<UserSummary>, AppError> {
        a4n_db::auth::list_users(self.database.executor()).await
    }

    pub async fn list_invite_code_records(&self) -> Result<Vec<InviteCodeSummary>, AppError> {
        a4n_db::auth::list_invite_codes(self.database.executor()).await
    }

    pub async fn update_auth_user_role(
        &self,
        user_id: i64,
        role: &str,
    ) -> Result<UserSummary, AppError> {
        a4n_db::auth::update_user_role_summary(self.database.executor(), user_id, role).await
    }
}

#[async_trait::async_trait]
impl AuthReadApplicationPort for AuthApplication {
    async fn current_user(&self, user_id: i64) -> Result<UserSummary, AppError> {
        a4n_db::auth::current_user_summary(self.database.executor(), user_id).await
    }

    async fn list_sessions(&self, user_id: i64) -> Result<Vec<SessionSummary>, AppError> {
        let user_id = required_session_user_id(user_id, "列出会话")?;
        let sessions =
            a4n_db::auth::list_session_records(self.database.executor(), user_id).await?;
        Ok(sessions.iter().map(session_summary).collect())
    }

    async fn verify_session_refresh_token(
        &self,
        user_id: i64,
        session_id: &str,
        refresh_token: &str,
    ) -> Result<(), AppError> {
        a4n_db::auth::verify_session_refresh_token_valid(
            self.database.executor(),
            user_id,
            session_id,
            refresh_token,
        )
        .await
    }

    async fn verify_active_session(&self, user_id: i64, session_id: &str) -> Result<(), AppError> {
        a4n_db::auth::verify_active_session_valid(self.database.executor(), user_id, session_id)
            .await
    }
}

#[async_trait::async_trait]
impl AuthMutationApplicationPort for AuthApplication {
    async fn require_existing_user_or_bootstrap_admin(
        &self,
        bootstrap: Option<(&str, &str, &str)>,
    ) -> Result<(), AppError> {
        let bootstrap = bootstrap.map(|(username, email, password)| BootstrapAdminInput {
            username,
            email,
            password,
        });
        a4n_db::auth::require_existing_user_or_bootstrap_admin(self.database.executor(), bootstrap)
            .await
    }

    async fn register_user(&self, input: RegisterInput) -> Result<UserSummary, AppError> {
        a4n_db::auth::register_user_with_invite_summary(
            self.database.executor(),
            &input.username,
            &input.email,
            &input.password,
            &input.invite_code,
        )
        .await
    }

    async fn login_user(&self, input: LoginInput) -> Result<UserSummary, AppError> {
        a4n_db::auth::login_user_summary(self.database.executor(), &input.username, &input.password)
            .await
    }

    async fn create_session(
        &self,
        user_id: i64,
        refresh_token: &str,
        user_agent: Option<&str>,
    ) -> Result<SessionSummary, AppError> {
        let session = a4n_db::auth::create_session_record(
            self.database.executor(),
            user_id,
            refresh_token,
            user_agent,
        )
        .await?;
        Ok(session_summary(&session))
    }

    async fn revoke_session(&self, user_id: i64, session_id: &str) -> Result<(), AppError> {
        let user_id = required_session_user_id(user_id, "撤销会话")?;
        let session_id = required_session_id(session_id, "撤销会话")?;
        a4n_db::auth::revoke_session(self.database.executor(), user_id, &session_id).await
    }

    async fn update_profile(
        &self,
        user_id: i64,
        input: UpdateProfileInput,
    ) -> Result<UserSummary, AppError> {
        let avatar_url = input.avatar_url.as_ref().map(|value| value.as_deref());
        a4n_db::auth::update_profile_summary(
            self.database.executor(),
            user_id,
            input.email.as_deref(),
            avatar_url,
            input.page_cli_enabled,
        )
        .await
    }

    async fn change_password(
        &self,
        user_id: i64,
        input: ChangePasswordInput,
    ) -> Result<(), AppError> {
        a4n_db::auth::change_password(
            self.database.executor(),
            user_id,
            &input.old_password,
            &input.new_password,
        )
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
        a4n_db::auth::rotate_session_refresh_token_for_user(
            self.database.executor(),
            user_id,
            &session_id,
            old_refresh_token,
            new_refresh_token,
        )
        .await
    }

    async fn login_with_oauth2(
        &self,
        provider: &str,
        provider_id: &str,
        email: Option<&str>,
        username_hint: Option<&str>,
        avatar_url: Option<&str>,
    ) -> Result<UserSummary, AppError> {
        a4n_db::auth::login_with_oauth2_summary(
            self.database.executor(),
            provider,
            provider_id,
            email,
            username_hint,
            avatar_url,
        )
        .await
    }
}

fn session_summary(session: &a4n_db::auth::UserSessionRecord) -> SessionSummary {
    SessionSummary {
        id: session.id.clone(),
        user_agent: session.user_agent.clone(),
        created_at: session.created_at.and_utc().to_rfc3339(),
        expires_at: session.expires_at.and_utc().to_rfc3339(),
        last_used_at: session
            .last_used_at
            .map(|value| value.and_utc().to_rfc3339()),
    }
}

#[cfg(test)]
mod tests {
    use super::session_summary;
    use a4n_db::auth::UserSessionRecord;

    #[test]
    fn session_timestamps_are_utc_instants() {
        let now = Default::default();
        let session = UserSessionRecord {
            id: "session".into(),
            user_agent: None,
            created_at: now,
            expires_at: now,
            last_used_at: Some(now),
        };
        let session = session_summary(&session);
        assert!(session.created_at.ends_with("+00:00"));
        assert!(session.expires_at.ends_with("+00:00"));
        assert!(session.last_used_at.unwrap().ends_with("+00:00"));
    }
}
