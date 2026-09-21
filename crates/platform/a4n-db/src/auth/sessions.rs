use super::*;

use super::rules::hash_refresh_token;
use super::users::active_user;

const SESSION_EXP_DAYS: i64 = 30;

pub async fn create_session(
    db: &DatabaseConnection,
    user_id: i64,
    refresh_token: &str,
    user_agent: Option<&str>,
) -> Result<user_session::Model, AppError> {
    active_user(db, user_id).await?;
    let now = Utc::now().naive_utc();
    user_session::ActiveModel {
        id: Set(Uuid::new_v4().to_string()),
        user_id: Set(user_id),
        refresh_token_hash: Set(hash_refresh_token(refresh_token)),
        user_agent: Set(user_agent.map(str::to_string)),
        is_revoked: Set(false),
        created_at: Set(now),
        expires_at: Set(now + Duration::days(SESSION_EXP_DAYS)),
        revoked_at: Set(None),
        last_used_at: Set(Some(now)),
    }
    .insert(db)
    .await
    .map_err(AppError::Database)
}

pub async fn verify_session_refresh_token(
    db: &DatabaseConnection,
    user_id: i64,
    session_id: &str,
    refresh_token: &str,
) -> Result<user_session::Model, AppError> {
    let session = verify_active_session(db, user_id, session_id).await?;
    if session.refresh_token_hash != hash_refresh_token(refresh_token) {
        return Err(AppError::Unauthorized("刷新令牌与会话不匹配".into()));
    }
    Ok(session)
}

pub async fn verify_active_session(
    db: &DatabaseConnection,
    user_id: i64,
    session_id: &str,
) -> Result<user_session::Model, AppError> {
    let now = Utc::now().naive_utc();
    let (session_result, user_result) = futures::join!(
        user_session::Entity::find_by_id(session_id.to_string())
            .filter(user_session::Column::UserId.eq(user_id))
            .one(db),
        user::Entity::find_by_id(user_id).one(db),
    );
    let session = session_result
        .map_err(AppError::Database)?
        .ok_or_else(|| AppError::Unauthorized("会话不存在或已失效".into()))?;

    if session.is_revoked || session.expires_at <= now {
        return Err(AppError::Unauthorized("会话已过期或已撤销".into()));
    }
    let user = user_result
        .map_err(AppError::Database)?
        .ok_or_else(|| AppError::Unauthorized("会话不存在或已失效".into()))?;
    if !user.is_active {
        return Err(AppError::Forbidden("账户已被禁用".into()));
    }
    Ok(session)
}

pub async fn rotate_session_refresh_token(
    db: &DatabaseConnection,
    session: user_session::Model,
    old_refresh_token: &str,
    new_refresh_token: &str,
) -> Result<user_session::Model, AppError> {
    let now = Utc::now().naive_utc();
    let session_id = session.id.clone();
    let updated = user_session::Entity::update_many()
        .col_expr(
            user_session::Column::RefreshTokenHash,
            Expr::value(hash_refresh_token(new_refresh_token)),
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
        .filter(user_session::Column::RefreshTokenHash.eq(hash_refresh_token(old_refresh_token)))
        .exec(db)
        .await
        .map_err(AppError::Database)?;
    if updated.rows_affected != 1 {
        return Err(AppError::Unauthorized(
            "刷新令牌已被轮换或会话已失效".into(),
        ));
    }
    user_session::Entity::find_by_id(session_id)
        .one(db)
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| AppError::Unauthorized("会话不存在或已失效".into()))
}

pub async fn list_sessions(
    db: &DatabaseConnection,
    user_id: i64,
) -> Result<Vec<user_session::Model>, AppError> {
    active_user(db, user_id).await?;
    let now = Utc::now().naive_utc();
    user_session::Entity::find()
        .filter(user_session::Column::UserId.eq(user_id))
        .filter(user_session::Column::IsRevoked.eq(false))
        .filter(user_session::Column::ExpiresAt.gt(now))
        .order_by_desc(user_session::Column::LastUsedAt)
        .all(db)
        .await
        .map_err(AppError::Database)
}

pub async fn revoke_session(
    db: &DatabaseConnection,
    user_id: i64,
    session_id: &str,
) -> Result<(), AppError> {
    let session = user_session::Entity::find_by_id(session_id.to_string())
        .filter(user_session::Column::UserId.eq(user_id))
        .one(db)
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| AppError::NotFound("会话不存在".into()))?;
    let mut active: user_session::ActiveModel = session.into();
    active.is_revoked = Set(true);
    active.revoked_at = Set(Some(Utc::now().naive_utc()));
    active.update(db).await.map_err(AppError::Database)?;
    Ok(())
}
