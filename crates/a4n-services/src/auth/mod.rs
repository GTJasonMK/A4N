//! 用户认证服务：注册、登录、密码管理
//!
//! 对应文档 12-用户认证系统

pub mod jwt;
pub mod oauth;
mod oauth_login;
mod session;

use argon2::password_hash::{SaltString, rand_core::OsRng};
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use chrono::Utc;
use sea_orm::sea_query::Expr;
use sea_orm::*;
use std::sync::Arc;

use a4n_core::error::AppError;
use a4n_core::models::user::{
    invite_code,
    role::{ROLE_ADMIN, ROLE_USER, validate_user_role},
    user,
};

#[derive(Clone)]
pub struct AuthService {
    db: Arc<DatabaseConnection>,
}

impl AuthService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 根据用户名查找用户
    pub async fn find_by_username(&self, username: &str) -> Result<Option<user::Model>, AppError> {
        let username = normalize_username(username)?;
        Self::find_by_username_on(self.db.as_ref(), &username).await
    }

    async fn find_by_username_on<C>(db: &C, username: &str) -> Result<Option<user::Model>, AppError>
    where
        C: ConnectionTrait,
    {
        user::Entity::find()
            .filter(user::Column::Username.eq(username))
            .one(db)
            .await
            .map_err(AppError::Database)
    }

    /// 根据邮箱查找用户
    pub async fn find_by_email(&self, email: &str) -> Result<Option<user::Model>, AppError> {
        let email = normalize_email(email)?;
        Self::find_by_email_on(self.db.as_ref(), &email).await
    }

    async fn find_by_email_on<C>(db: &C, email: &str) -> Result<Option<user::Model>, AppError>
    where
        C: ConnectionTrait,
    {
        user::Entity::find()
            .filter(user::Column::Email.eq(email))
            .one(db)
            .await
            .map_err(AppError::Database)
    }

    /// 获取用户
    pub async fn get(&self, user_id: i64) -> Result<user::Model, AppError> {
        user::Entity::find_by_id(user_id)
            .one(self.db.as_ref())
            .await
            .map_err(AppError::Database)?
            .ok_or_else(|| AppError::NotFound(format!("用户不存在: {user_id}")))
    }

    /// 获取当前登录用户。禁用账户不能被恢复为有效当前用户。
    pub async fn get_current_user(&self, user_id: i64) -> Result<user::Model, AppError> {
        self.get_active(user_id).await
    }

    async fn get_active(&self, user_id: i64) -> Result<user::Model, AppError> {
        let user = self.get(user_id).await?;
        if !user.is_active {
            return Err(AppError::Forbidden("账户已被禁用".into()));
        }
        Ok(user)
    }

    /// 注册新用户
    pub async fn register(
        &self,
        username: &str,
        email: &str,
        password: &str,
    ) -> Result<user::Model, AppError> {
        self.create_user(username, email, password, ROLE_USER).await
    }

    /// 使用一次性邀请码注册公开用户。
    pub async fn register_with_invite(
        &self,
        username: &str,
        email: &str,
        password: &str,
        raw_invite_code: &str,
    ) -> Result<user::Model, AppError> {
        let code = normalize_invite_code(raw_invite_code)?;
        let txn = self.db.begin().await.map_err(AppError::Database)?;

        let result = async {
            let invite = invite_code::Entity::find()
                .filter(invite_code::Column::Code.eq(code))
                .one(&txn)
                .await
                .map_err(AppError::Database)?
                .ok_or_else(|| AppError::InvalidArgument("邀请码无效".into()))?;
            if invite.used_at.is_some() || invite.used_by_user_id.is_some() {
                return Err(AppError::InvalidArgument("邀请码已被使用".into()));
            }

            let user = self
                .create_user_on(&txn, username, email, password, ROLE_USER)
                .await?;
            let now = Utc::now().naive_utc();
            let updated = invite_code::Entity::update_many()
                .col_expr(
                    invite_code::Column::UsedByUserId,
                    Expr::value(Some(user.id)),
                )
                .col_expr(invite_code::Column::UsedAt, Expr::value(Some(now)))
                .filter(invite_code::Column::Id.eq(invite.id))
                .filter(invite_code::Column::UsedAt.is_null())
                .filter(invite_code::Column::UsedByUserId.is_null())
                .exec(&txn)
                .await
                .map_err(AppError::Database)?;
            if updated.rows_affected != 1 {
                return Err(AppError::InvalidArgument("邀请码已被使用".into()));
            }
            Ok(user)
        }
        .await;

        match result {
            Ok(user) => {
                txn.commit().await.map_err(AppError::Database)?;
                Ok(user)
            }
            Err(err) => {
                txn.rollback().await.map_err(AppError::Database)?;
                Err(err)
            }
        }
    }

    pub async fn create_invite_code(
        &self,
        code: &str,
        created_by_user_id: Option<i64>,
    ) -> Result<invite_code::Model, AppError> {
        let code = normalize_invite_code(code)?;
        if let Some(user_id) = created_by_user_id {
            validate_positive_user_id(user_id)?;
        }
        if invite_code::Entity::find()
            .filter(invite_code::Column::Code.eq(code.clone()))
            .one(self.db.as_ref())
            .await
            .map_err(AppError::Database)?
            .is_some()
        {
            return Err(AppError::InvalidArgument("邀请码已存在".into()));
        }
        let now = Utc::now().naive_utc();
        invite_code::ActiveModel {
            code: Set(code),
            created_by_user_id: Set(created_by_user_id),
            used_by_user_id: Set(None),
            created_at: Set(now),
            used_at: Set(None),
            ..Default::default()
        }
        .insert(self.db.as_ref())
        .await
        .map_err(AppError::Database)
    }

    pub async fn list_invite_codes(&self) -> Result<Vec<invite_code::Model>, AppError> {
        invite_code::Entity::find()
            .order_by_desc(invite_code::Column::CreatedAt)
            .order_by_desc(invite_code::Column::Id)
            .all(self.db.as_ref())
            .await
            .map_err(AppError::Database)
    }

    /// 创建指定角色的用户。调用方必须在服务层外完成权限判断。
    pub async fn create_user(
        &self,
        username: &str,
        email: &str,
        password: &str,
        role: &str,
    ) -> Result<user::Model, AppError> {
        self.create_user_on(self.db.as_ref(), username, email, password, role)
            .await
    }

    async fn create_user_on<C>(
        &self,
        db: &C,
        username: &str,
        email: &str,
        password: &str,
        role: &str,
    ) -> Result<user::Model, AppError>
    where
        C: ConnectionTrait,
    {
        let username = normalize_username(username)?;
        let email = normalize_email(email)?;
        validate_user_role(role)?;
        if Self::find_by_username_on(db, &username).await?.is_some() {
            return Err(AppError::InvalidArgument("用户名已被占用".into()));
        }
        if Self::find_by_email_on(db, &email).await?.is_some() {
            return Err(AppError::InvalidArgument("邮箱已被注册".into()));
        }
        validate_password_for_storage(password)?;

        let password_hash = Self::hash_password(password)?;
        let now = Utc::now().naive_utc();

        user::ActiveModel {
            username: Set(username),
            email: Set(email),
            password_hash: Set(password_hash),
            role: Set(role.to_string()),
            oauth_provider: Set(None),
            oauth_subject: Set(None),
            avatar_url: Set(None),
            is_active: Set(true),
            created_at: Set(now),
            updated_at: Set(now),
            last_login_at: Set(None),
            ..Default::default()
        }
        .insert(db)
        .await
        .map_err(AppError::Database)
    }

    pub async fn bootstrap_first_admin(
        &self,
        username: &str,
        email: &str,
        password: &str,
    ) -> Result<Option<user::Model>, AppError> {
        if self.count_users().await? != 0 {
            return Ok(None);
        }
        self.create_user(username, email, password, ROLE_ADMIN)
            .await
            .map(Some)
    }

    pub async fn require_existing_user_or_bootstrap_admin(
        &self,
        bootstrap: Option<BootstrapAdminInput<'_>>,
    ) -> Result<(), AppError> {
        if self.count_users().await? != 0 {
            return Ok(());
        }
        let input = bootstrap.ok_or_else(|| {
            AppError::ConfigError(
                "用户表为空，Web 部署必须设置 A4N_BOOTSTRAP_ADMIN_USERNAME、A4N_BOOTSTRAP_ADMIN_EMAIL、A4N_BOOTSTRAP_ADMIN_PASSWORD 创建首个管理员"
                    .into(),
            )
        })?;
        self.bootstrap_first_admin(input.username, input.email, input.password)
            .await?;
        Ok(())
    }

    pub async fn count_users(&self) -> Result<u64, AppError> {
        user::Entity::find()
            .count(self.db.as_ref())
            .await
            .map_err(AppError::Database)
    }

    pub async fn list_users(&self) -> Result<Vec<user::Model>, AppError> {
        user::Entity::find()
            .order_by_asc(user::Column::Id)
            .all(self.db.as_ref())
            .await
            .map_err(AppError::Database)
    }

    pub async fn update_user_role(
        &self,
        user_id: i64,
        role: &str,
    ) -> Result<user::Model, AppError> {
        validate_positive_user_id(user_id)?;
        validate_user_role(role)?;

        let u = self.get(user_id).await?;
        let mut active: user::ActiveModel = u.into();
        active.role = Set(role.to_string());
        active.updated_at = Set(Utc::now().naive_utc());
        active
            .update(self.db.as_ref())
            .await
            .map_err(AppError::Database)
    }

    pub async fn disable_user(&self, caller_user_id: i64, user_id: i64) -> Result<(), AppError> {
        validate_positive_user_id(user_id)?;
        if caller_user_id == user_id {
            return Err(AppError::InvalidArgument("不能禁用当前登录用户".into()));
        }

        let u = self.get(user_id).await?;
        let mut active: user::ActiveModel = u.into();
        active.is_active = Set(false);
        active.updated_at = Set(Utc::now().naive_utc());
        active
            .update(self.db.as_ref())
            .await
            .map_err(AppError::Database)?;
        Ok(())
    }

    /// 密码登录验证
    pub async fn login(&self, username: &str, password: &str) -> Result<user::Model, AppError> {
        let username = normalize_username(username)?;
        let u = self
            .find_by_username(&username)
            .await?
            .ok_or_else(|| AppError::Forbidden("用户名或密码错误".into()))?;

        if !u.is_active {
            return Err(AppError::Forbidden("账户已被禁用".into()));
        }

        if !Self::verify_password(password, &u.password_hash)? {
            return Err(AppError::Forbidden("用户名或密码错误".into()));
        }

        let now = Utc::now().naive_utc();
        let mut active: user::ActiveModel = u.clone().into();
        active.last_login_at = Set(Some(now));
        active
            .update(self.db.as_ref())
            .await
            .map_err(AppError::Database)
    }

    /// 更新用户信息
    pub async fn update_profile(
        &self,
        user_id: i64,
        email: Option<&str>,
        avatar_url: Option<Option<&str>>,
    ) -> Result<user::Model, AppError> {
        let u = self.get_active(user_id).await?;
        let mut active: user::ActiveModel = u.into();
        if let Some(v) = email {
            let email = normalize_email(v)?;
            if let Some(existing) = self.find_by_email(&email).await?
                && existing.id != user_id
            {
                return Err(AppError::InvalidArgument("邮箱已被注册".into()));
            }
            active.email = Set(email);
        }
        if let Some(v) = avatar_url {
            active.avatar_url = Set(v.and_then(normalize_optional_text));
        }
        active.updated_at = Set(Utc::now().naive_utc());
        active
            .update(self.db.as_ref())
            .await
            .map_err(AppError::Database)
    }

    /// 修改密码
    pub async fn change_password(
        &self,
        user_id: i64,
        old_password: &str,
        new_password: &str,
    ) -> Result<(), AppError> {
        let u = self.get_active(user_id).await?;
        if !Self::verify_password(old_password, &u.password_hash)? {
            return Err(AppError::Forbidden("原密码错误".into()));
        }
        validate_password_for_storage(new_password)?;
        let new_hash = Self::hash_password(new_password)?;
        let mut active: user::ActiveModel = u.into();
        active.password_hash = Set(new_hash);
        active.updated_at = Set(Utc::now().naive_utc());
        active
            .update(self.db.as_ref())
            .await
            .map_err(AppError::Database)?;
        Ok(())
    }

    fn hash_password(password: &str) -> Result<String, AppError> {
        let salt = SaltString::generate(&mut OsRng);
        Argon2::default()
            .hash_password(password.as_bytes(), &salt)
            .map(|hash| hash.to_string())
            .map_err(|e| AppError::Internal(anyhow::anyhow!("密码哈希失败: {e}")))
    }

    fn verify_password(password: &str, hash: &str) -> Result<bool, AppError> {
        let parsed = PasswordHash::new(hash)
            .map_err(|e| AppError::Internal(anyhow::anyhow!("密码哈希格式无效: {e}")))?;
        Ok(Argon2::default()
            .verify_password(password.as_bytes(), &parsed)
            .is_ok())
    }
}

#[derive(Debug, Clone, Copy)]
pub struct BootstrapAdminInput<'a> {
    pub username: &'a str,
    pub email: &'a str,
    pub password: &'a str,
}

fn normalize_username(username: &str) -> Result<String, AppError> {
    let username = username.trim();
    if username.is_empty() {
        return Err(AppError::InvalidArgument("用户名不能为空".into()));
    }
    Ok(username.to_string())
}

fn normalize_email(email: &str) -> Result<String, AppError> {
    let email = email.trim();
    validate_email_identity(email)?;
    Ok(email.to_string())
}

fn normalize_optional_text(value: &str) -> Option<String> {
    let value = value.trim();
    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}

fn normalize_invite_code(code: &str) -> Result<String, AppError> {
    let code = code.trim();
    if code.is_empty() {
        return Err(AppError::InvalidArgument("邀请码不能为空".into()));
    }
    Ok(code.to_string())
}

fn validate_email_identity(email: &str) -> Result<(), AppError> {
    let Some((local, domain)) = email.split_once('@') else {
        return Err(AppError::InvalidArgument("email 格式无效".into()));
    };
    if email.chars().any(char::is_whitespace)
        || local.is_empty()
        || domain.is_empty()
        || domain.contains('@')
    {
        return Err(AppError::InvalidArgument("email 格式无效".into()));
    }
    Ok(())
}

fn validate_password_for_storage(password: &str) -> Result<(), AppError> {
    if password.trim().is_empty() {
        return Err(AppError::InvalidArgument("密码不能为空".into()));
    }
    Ok(())
}

fn validate_positive_user_id(user_id: i64) -> Result<(), AppError> {
    if user_id <= 0 {
        Err(AppError::InvalidArgument("user_id 必须大于 0".into()))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests;
