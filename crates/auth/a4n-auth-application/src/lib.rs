use a4n_api_contract::auth::{
    AuthMutationApplicationPort, AuthReadApplicationPort, ChangePasswordInput, LoginInput,
    RegisterInput, SessionSummary, UpdateProfileInput, UserSummary, required_session_id,
    required_session_user_id,
};
use a4n_db::{DbHandle, auth::BootstrapAdminInput};
use a4n_db_models::models::user::{user, user_session};
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
        a4n_db::auth::user_summary_by_id(self.database.connection(), user_id).await
    }

    pub async fn auth_user_authorization_by_id(
        &self,
        user_id: i64,
    ) -> Result<a4n_domain::auth::UserAuthorizationRecord, AppError> {
        a4n_db::auth::user_authorization_by_id(self.database.connection(), user_id).await
    }

    pub async fn create_auth_user(
        &self,
        username: &str,
        email: &str,
        password: &str,
        role: &str,
    ) -> Result<UserSummary, AppError> {
        let user =
            a4n_db::auth::create_user(self.database.connection(), username, email, password, role)
                .await?;
        Ok(user_summary(&user))
    }

    pub async fn create_invite_code_record(
        &self,
        code: &str,
        created_by_user_id: Option<i64>,
    ) -> Result<InviteCodeSummary, AppError> {
        a4n_db::auth::create_invite_code(self.database.connection(), code, created_by_user_id).await
    }

    pub async fn list_auth_users(&self) -> Result<Vec<UserSummary>, AppError> {
        a4n_db::auth::list_users(self.database.connection()).await
    }

    pub async fn list_invite_code_records(&self) -> Result<Vec<InviteCodeSummary>, AppError> {
        a4n_db::auth::list_invite_codes(self.database.connection()).await
    }

    pub async fn update_auth_user_role(
        &self,
        user_id: i64,
        role: &str,
    ) -> Result<UserSummary, AppError> {
        let user =
            a4n_db::auth::update_user_role(self.database.connection(), user_id, role).await?;
        Ok(user_summary(&user))
    }
}

#[async_trait::async_trait]
impl AuthReadApplicationPort for AuthApplication {
    async fn current_user(&self, user_id: i64) -> Result<UserSummary, AppError> {
        a4n_db::auth::current_user_summary(self.database.connection(), user_id).await
    }

    async fn list_sessions(&self, user_id: i64) -> Result<Vec<SessionSummary>, AppError> {
        let user_id = required_session_user_id(user_id, "列出会话")?;
        let sessions = a4n_db::auth::list_sessions(self.database.connection(), user_id).await?;
        Ok(sessions.iter().map(session_summary).collect())
    }

    async fn verify_session_refresh_token(
        &self,
        user_id: i64,
        session_id: &str,
        refresh_token: &str,
    ) -> Result<(), AppError> {
        a4n_db::auth::verify_session_refresh_token(
            self.database.connection(),
            user_id,
            session_id,
            refresh_token,
        )
        .await
        .map(|_| ())
    }

    async fn verify_active_session(&self, user_id: i64, session_id: &str) -> Result<(), AppError> {
        a4n_db::auth::verify_active_session(self.database.connection(), user_id, session_id)
            .await
            .map(|_| ())
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
        a4n_db::auth::require_existing_user_or_bootstrap_admin(
            self.database.connection(),
            bootstrap,
        )
        .await
    }

    async fn register_user(&self, input: RegisterInput) -> Result<UserSummary, AppError> {
        let user = a4n_db::auth::register_user_with_invite(
            self.database.connection(),
            &input.username,
            &input.email,
            &input.password,
            &input.invite_code,
        )
        .await?;
        Ok(user_summary(&user))
    }

    async fn login_user(&self, input: LoginInput) -> Result<UserSummary, AppError> {
        let user =
            a4n_db::auth::login_user(self.database.connection(), &input.username, &input.password)
                .await?;
        Ok(user_summary(&user))
    }

    async fn create_session(
        &self,
        user_id: i64,
        refresh_token: &str,
        user_agent: Option<&str>,
    ) -> Result<SessionSummary, AppError> {
        let session = a4n_db::auth::create_session(
            self.database.connection(),
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
        a4n_db::auth::revoke_session(self.database.connection(), user_id, &session_id).await
    }

    async fn update_profile(
        &self,
        user_id: i64,
        input: UpdateProfileInput,
    ) -> Result<UserSummary, AppError> {
        let avatar_url = input.avatar_url.as_ref().map(|value| value.as_deref());
        let user = a4n_db::auth::update_profile(
            self.database.connection(),
            user_id,
            input.email.as_deref(),
            avatar_url,
            input.page_cli_enabled,
        )
        .await?;
        Ok(user_summary(&user))
    }

    async fn change_password(
        &self,
        user_id: i64,
        input: ChangePasswordInput,
    ) -> Result<(), AppError> {
        a4n_db::auth::change_password(
            self.database.connection(),
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
        let session = a4n_db::auth::verify_session_refresh_token(
            self.database.connection(),
            user_id,
            &session_id,
            old_refresh_token,
        )
        .await?;
        a4n_db::auth::rotate_session_refresh_token(
            self.database.connection(),
            session,
            old_refresh_token,
            new_refresh_token,
        )
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
    ) -> Result<UserSummary, AppError> {
        let user = a4n_db::auth::login_with_oauth2(
            self.database.connection(),
            provider,
            provider_id,
            email,
            username_hint,
            avatar_url,
        )
        .await?;
        Ok(user_summary(&user))
    }
}

fn user_summary(user: &user::Model) -> UserSummary {
    UserSummary {
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

fn session_summary(session: &user_session::Model) -> SessionSummary {
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
    use super::{session_summary, user_summary};
    use a4n_db_models::models::user::{user, user_session};

    #[test]
    fn user_summary_never_serializes_database_secrets() {
        let now = Default::default();
        let record = user::Model {
            id: 12,
            username: "reader".into(),
            email: "reader@example.test".into(),
            password_hash: "password-hash-secret".into(),
            role: "user".into(),
            oauth_provider: None,
            oauth_subject: None,
            avatar_url: None,
            page_cli_enabled: false,
            is_active: true,
            lifecycle_gate: "active".into(),
            created_at: now,
            updated_at: now,
            last_login_at: None,
        };

        let summary = user_summary(&record);
        let serialized = serde_json::to_value(summary).unwrap();

        assert_eq!(serialized["id"], 12);
        assert_eq!(serialized["username"], "reader");
        assert!(
            serialized["created_at"]
                .as_str()
                .unwrap()
                .ends_with("+00:00")
        );
        assert!(serialized.get("password_hash").is_none());
        assert!(serialized.get("lifecycle_gate").is_none());
        assert!(!serialized.to_string().contains("password-hash-secret"));
    }

    #[test]
    fn session_timestamps_are_utc_instants() {
        let now = Default::default();
        let session = user_session::Model {
            id: "session".into(),
            user_id: 7,
            refresh_token_hash: "hash".into(),
            user_agent: None,
            is_revoked: false,
            created_at: now,
            expires_at: now,
            revoked_at: None,
            last_used_at: Some(now),
        };
        let session = session_summary(&session);
        assert!(session.created_at.ends_with("+00:00"));
        assert!(session.expires_at.ends_with("+00:00"));
        assert!(session.last_used_at.unwrap().ends_with("+00:00"));
    }
}
