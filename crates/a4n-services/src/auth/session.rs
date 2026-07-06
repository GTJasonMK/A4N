use chrono::{Duration, Utc};
use sea_orm::sea_query::Expr;
use sea_orm::*;
use sha2::{Digest, Sha256};
use uuid::Uuid;

use a4n_core::error::AppError;
use a4n_core::models::user::{user, user_session};

use super::AuthService;

const SESSION_EXP_DAYS: i64 = 30;

impl AuthService {
    fn hash_refresh_token(token: &str) -> String {
        let digest = Sha256::digest(token.as_bytes());
        let mut out = String::with_capacity(digest.len() * 2);
        for byte in digest {
            use std::fmt::Write as _;
            let _ = write!(&mut out, "{byte:02x}");
        }
        out
    }

    /// 创建可撤销会话。refresh token 只保存 SHA-256 摘要。
    pub async fn create_session(
        &self,
        user_id: i64,
        refresh_token: &str,
        user_agent: Option<&str>,
    ) -> Result<user_session::Model, AppError> {
        self.get_active(user_id).await?;
        let now = Utc::now().naive_utc();
        user_session::ActiveModel {
            id: Set(Uuid::new_v4().to_string()),
            user_id: Set(user_id),
            refresh_token_hash: Set(Self::hash_refresh_token(refresh_token)),
            user_agent: Set(user_agent.map(|s| s.to_string())),
            is_revoked: Set(false),
            created_at: Set(now),
            expires_at: Set(now + Duration::days(SESSION_EXP_DAYS)),
            revoked_at: Set(None),
            last_used_at: Set(Some(now)),
        }
        .insert(self.db.as_ref())
        .await
        .map_err(AppError::Database)
    }

    /// 校验 refresh token 是否仍绑定到一个活跃会话。
    pub async fn verify_session_refresh_token(
        &self,
        user_id: i64,
        session_id: &str,
        refresh_token: &str,
    ) -> Result<user_session::Model, AppError> {
        let now = Utc::now().naive_utc();
        let session = user_session::Entity::find_by_id(session_id.to_string())
            .filter(user_session::Column::UserId.eq(user_id))
            .one(self.db.as_ref())
            .await
            .map_err(AppError::Database)?
            .ok_or_else(|| AppError::Forbidden("会话不存在或已失效".into()))?;

        if session.is_revoked || session.expires_at <= now {
            return Err(AppError::Forbidden("会话已过期或已撤销".into()));
        }
        if session.refresh_token_hash != Self::hash_refresh_token(refresh_token) {
            return Err(AppError::Forbidden("刷新令牌与会话不匹配".into()));
        }
        let user = user::Entity::find_by_id(user_id)
            .one(self.db.as_ref())
            .await
            .map_err(AppError::Database)?
            .ok_or_else(|| AppError::Forbidden("会话不存在或已失效".into()))?;
        if !user.is_active {
            return Err(AppError::Forbidden("账户已被禁用".into()));
        }
        Ok(session)
    }

    /// 刷新令牌成功后轮换会话中保存的 refresh token 摘要。
    pub async fn rotate_session_refresh_token(
        &self,
        session: user_session::Model,
        old_refresh_token: &str,
        new_refresh_token: &str,
    ) -> Result<user_session::Model, AppError> {
        let now = Utc::now().naive_utc();
        let session_id = session.id.clone();
        let updated = user_session::Entity::update_many()
            .col_expr(
                user_session::Column::RefreshTokenHash,
                Expr::value(Self::hash_refresh_token(new_refresh_token)),
            )
            .col_expr(
                user_session::Column::ExpiresAt,
                Expr::value(now + Duration::days(SESSION_EXP_DAYS)),
            )
            .col_expr(user_session::Column::LastUsedAt, Expr::value(Some(now)))
            .filter(user_session::Column::Id.eq(session_id.clone()))
            .filter(user_session::Column::UserId.eq(session.user_id))
            .filter(user_session::Column::IsRevoked.eq(false))
            .filter(user_session::Column::ExpiresAt.gt(now))
            .filter(
                user_session::Column::RefreshTokenHash
                    .eq(Self::hash_refresh_token(old_refresh_token)),
            )
            .exec(self.db.as_ref())
            .await
            .map_err(AppError::Database)?;
        if updated.rows_affected != 1 {
            return Err(AppError::Forbidden("刷新令牌已被轮换或会话已失效".into()));
        }
        user_session::Entity::find_by_id(session_id)
            .one(self.db.as_ref())
            .await
            .map_err(AppError::Database)?
            .ok_or_else(|| AppError::Forbidden("会话不存在或已失效".into()))
    }

    pub async fn list_sessions(&self, user_id: i64) -> Result<Vec<user_session::Model>, AppError> {
        self.get_active(user_id).await?;
        let now = Utc::now().naive_utc();
        user_session::Entity::find()
            .filter(user_session::Column::UserId.eq(user_id))
            .filter(user_session::Column::IsRevoked.eq(false))
            .filter(user_session::Column::ExpiresAt.gt(now))
            .order_by_desc(user_session::Column::LastUsedAt)
            .all(self.db.as_ref())
            .await
            .map_err(AppError::Database)
    }

    pub async fn revoke_session(&self, user_id: i64, session_id: &str) -> Result<(), AppError> {
        let session = user_session::Entity::find_by_id(session_id.to_string())
            .filter(user_session::Column::UserId.eq(user_id))
            .one(self.db.as_ref())
            .await
            .map_err(AppError::Database)?
            .ok_or_else(|| AppError::NotFound("会话不存在".into()))?;
        let mut active: user_session::ActiveModel = session.into();
        active.is_revoked = Set(true);
        active.revoked_at = Set(Some(Utc::now().naive_utc()));
        active
            .update(self.db.as_ref())
            .await
            .map_err(AppError::Database)?;
        Ok(())
    }
}
