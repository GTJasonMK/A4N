use a4n_core::error::AppError;
use a4n_core::models::user::{invite_code, user, user_session};
use a4n_db::DbHandle;

use super::OrchestratorContext;
use crate::auth::{
    BootstrapAdminInput, ensure_same_user, profile_email_update, required_logout_identity,
    required_session_id, required_session_user_id,
};

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

    async fn current_user(&self, user_id: i64) -> Result<user::Model, AppError> {
        a4n_db::auth::current_user(self.database.connection(), user_id).await
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
    ) -> Result<user::Model, AppError> {
        a4n_db::auth::update_profile(self.database.connection(), user_id, email, avatar_url).await
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

    /// 登出：校验 user/session 身份后撤销会话。
    async fn logout_user_session(
        &self,
        user_id: Option<i64>,
        session_id: Option<&str>,
    ) -> Result<(), AppError> {
        let (user_id, session_id) = required_logout_identity(user_id, session_id)?;
        self.revoke_user_session(user_id, &session_id).await
    }

    /// 列出本人会话：同用户门禁 + 正数 user_id。
    async fn list_own_sessions(
        &self,
        caller_user_id: i64,
        target_user_id: i64,
    ) -> Result<Vec<user_session::Model>, AppError> {
        ensure_same_user(caller_user_id, target_user_id, "只能列出当前登录用户会话")?;
        let user_id = required_session_user_id(target_user_id, "列出会话")?;
        self.user_sessions(user_id).await
    }

    /// 撤销本人会话：同用户门禁 + 会话 ID 校验。
    async fn revoke_own_session(
        &self,
        caller_user_id: i64,
        target_user_id: i64,
        session_id: &str,
    ) -> Result<(), AppError> {
        ensure_same_user(caller_user_id, target_user_id, "只能撤销当前登录用户会话")?;
        let user_id = required_session_user_id(target_user_id, "撤销会话")?;
        let session_id = required_session_id(session_id, "撤销会话")?;
        self.revoke_user_session(user_id, &session_id).await
    }

    /// 更新本人资料：同用户门禁 + email 更新语义。
    async fn update_own_user_profile(
        &self,
        caller_user_id: i64,
        target_user_id: i64,
        email: Option<&Option<String>>,
        avatar_url: Option<Option<&str>>,
    ) -> Result<user::Model, AppError> {
        ensure_same_user(caller_user_id, target_user_id, "只能更新当前登录用户资料")?;
        let email = profile_email_update(email)?;
        self.update_user_profile(target_user_id, email, avatar_url)
            .await
    }

    /// 修改本人密码：同用户门禁后改密。
    async fn change_own_user_password(
        &self,
        caller_user_id: i64,
        target_user_id: i64,
        old_password: &str,
        new_password: &str,
    ) -> Result<(), AppError> {
        ensure_same_user(caller_user_id, target_user_id, "只能修改当前登录用户密码")?;
        self.change_user_password(target_user_id, old_password, new_password)
            .await
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

    async fn user_sessions(&self, user_id: i64) -> Result<Vec<user_session::Model>, AppError> {
        a4n_db::auth::list_sessions(self.database.connection(), user_id).await
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
    pub async fn auth_user_by_id(&self, user_id: i64) -> Result<user::Model, AppError> {
        self.auth.user_by_id(user_id).await
    }

    pub async fn current_user(&self, user_id: i64) -> Result<user::Model, AppError> {
        self.auth.current_user(user_id).await
    }

    pub async fn require_existing_user_or_bootstrap_admin(
        &self,
        bootstrap: Option<BootstrapAdminInput<'_>>,
    ) -> Result<(), AppError> {
        self.auth
            .require_existing_user_or_bootstrap_admin(bootstrap)
            .await
    }

    pub async fn register_user_with_invite(
        &self,
        username: &str,
        email: &str,
        password: &str,
        invite_code: &str,
    ) -> Result<user::Model, AppError> {
        self.auth
            .register_user_with_invite(username, email, password, invite_code)
            .await
    }

    pub async fn login_user(
        &self,
        username: &str,
        password: &str,
    ) -> Result<user::Model, AppError> {
        self.auth.login_user(username, password).await
    }

    pub async fn create_session(
        &self,
        user_id: i64,
        refresh_token: &str,
        user_agent: Option<&str>,
    ) -> Result<user_session::Model, AppError> {
        self.auth
            .create_session(user_id, refresh_token, user_agent)
            .await
    }

    pub async fn logout_user_session(
        &self,
        user_id: Option<i64>,
        session_id: Option<&str>,
    ) -> Result<(), AppError> {
        self.auth.logout_user_session(user_id, session_id).await
    }

    pub async fn list_own_sessions(
        &self,
        caller_user_id: i64,
        target_user_id: i64,
    ) -> Result<Vec<user_session::Model>, AppError> {
        self.auth
            .list_own_sessions(caller_user_id, target_user_id)
            .await
    }

    pub async fn revoke_own_session(
        &self,
        caller_user_id: i64,
        target_user_id: i64,
        session_id: &str,
    ) -> Result<(), AppError> {
        self.auth
            .revoke_own_session(caller_user_id, target_user_id, session_id)
            .await
    }

    pub async fn update_own_user_profile(
        &self,
        caller_user_id: i64,
        target_user_id: i64,
        email: Option<&Option<String>>,
        avatar_url: Option<Option<&str>>,
    ) -> Result<user::Model, AppError> {
        self.auth
            .update_own_user_profile(caller_user_id, target_user_id, email, avatar_url)
            .await
    }

    pub async fn change_own_user_password(
        &self,
        caller_user_id: i64,
        target_user_id: i64,
        old_password: &str,
        new_password: &str,
    ) -> Result<(), AppError> {
        self.auth
            .change_own_user_password(caller_user_id, target_user_id, old_password, new_password)
            .await
    }

    pub async fn verify_session_refresh_token(
        &self,
        user_id: i64,
        session_id: &str,
        refresh_token: &str,
    ) -> Result<user_session::Model, AppError> {
        self.auth
            .verify_session_refresh_token(user_id, session_id, refresh_token)
            .await
    }

    pub async fn rotate_session_refresh_token(
        &self,
        session: user_session::Model,
        old_refresh_token: &str,
        new_refresh_token: &str,
    ) -> Result<user_session::Model, AppError> {
        self.auth
            .rotate_session_refresh_token(session, old_refresh_token, new_refresh_token)
            .await
    }

    pub async fn create_auth_user(
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

    pub async fn create_invite_code_record(
        &self,
        code: &str,
        created_by_user_id: Option<i64>,
    ) -> Result<invite_code::Model, AppError> {
        self.auth
            .create_invite_code_record(code, created_by_user_id)
            .await
    }

    pub async fn update_auth_user_role(
        &self,
        user_id: i64,
        role: &str,
    ) -> Result<user::Model, AppError> {
        self.auth.update_auth_user_role(user_id, role).await
    }

    pub async fn login_with_oauth2(
        &self,
        provider: &str,
        provider_id: &str,
        email: Option<&str>,
        username_hint: Option<&str>,
        avatar_url: Option<&str>,
    ) -> Result<user::Model, AppError> {
        self.auth
            .login_with_oauth2(provider, provider_id, email, username_hint, avatar_url)
            .await
    }
}

impl OrchestratorContext {
    pub async fn user_sessions(
        &self,
        user_id: i64,
    ) -> Result<Vec<a4n_core::models::user::user_session::Model>, a4n_core::error::AppError> {
        self.auth.user_sessions(user_id).await
    }

    pub async fn revoke_user_session(
        &self,
        user_id: i64,
        session_id: &str,
    ) -> Result<(), a4n_core::error::AppError> {
        self.auth.revoke_user_session(user_id, session_id).await
    }

    pub async fn update_user_profile(
        &self,
        user_id: i64,
        email: Option<&str>,
        avatar_url: Option<Option<&str>>,
    ) -> Result<a4n_core::models::user::user::Model, a4n_core::error::AppError> {
        self.auth
            .update_user_profile(user_id, email, avatar_url)
            .await
    }

    pub async fn change_user_password(
        &self,
        user_id: i64,
        old_password: &str,
        new_password: &str,
    ) -> Result<(), a4n_core::error::AppError> {
        self.auth
            .change_user_password(user_id, old_password, new_password)
            .await
    }
}
