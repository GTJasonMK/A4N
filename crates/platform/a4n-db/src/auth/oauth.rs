use super::*;

use super::rules::{
    normalize_oauth_provider, normalize_oauth_subject, oauth_username_base, required_oauth_email,
};
use super::users::{find_by_email_on, find_by_username_on, user_by_id};
use a4n_domain::auth::ROLE_USER;
use sea_orm::QuerySelect;
use std::collections::HashSet;

pub async fn login_with_oauth2(
    db: &DatabaseConnection,
    provider: &str,
    provider_id: &str,
    email: Option<&str>,
    username_hint: Option<&str>,
    avatar_url: Option<&str>,
) -> Result<user::Model, AppError> {
    let provider = normalize_oauth_provider(provider)?;
    let provider_id = normalize_oauth_subject(provider_id)?;
    let now = Utc::now().naive_utc();

    if let Some(user) = user::Entity::find()
        .filter(user::Column::OauthProvider.eq(provider.as_str()))
        .filter(user::Column::OauthSubject.eq(provider_id.as_str()))
        .one(db)
        .await
        .map_err(AppError::Database)?
    {
        if !user.is_active {
            return Err(AppError::Unauthorized("账户已被禁用".into()));
        }
        let id = user.id;
        let mut active: user::ActiveModel = user.into();
        active.last_login_at = Set(Some(now));
        if let Some(value) = avatar_url {
            active.avatar_url = Set(Some(value.to_string()));
        }
        active.update(db).await.map_err(AppError::Database)?;
        return user_by_id(db, id).await;
    }

    let email_addr = required_oauth_email(email)?;
    if let Some(user) = find_by_email_on(db, email_addr).await? {
        if !user.is_active {
            return Err(AppError::Unauthorized("账户已被禁用".into()));
        }
        let id = user.id;
        let mut active: user::ActiveModel = user.into();
        active.oauth_provider = Set(Some(provider.clone()));
        active.oauth_subject = Set(Some(provider_id.clone()));
        active.last_login_at = Set(Some(now));
        if let Some(value) = avatar_url {
            active.avatar_url = Set(Some(value.to_string()));
        }
        active.updated_at = Set(now);
        active.update(db).await.map_err(AppError::Database)?;
        return user_by_id(db, id).await;
    }

    let base = oauth_username_base(username_hint, email_addr)?;
    let final_username = pick_unique_username(db, &base).await?;
    user::ActiveModel {
        username: Set(final_username),
        email: Set(email_addr.to_string()),
        password_hash: Set(String::new()),
        role: Set(ROLE_USER.to_string()),
        oauth_provider: Set(Some(provider)),
        oauth_subject: Set(Some(provider_id)),
        avatar_url: Set(avatar_url.map(str::to_string)),
        page_cli_enabled: Set(false),
        is_active: Set(true),
        created_at: Set(now),
        updated_at: Set(now),
        last_login_at: Set(Some(now)),
        ..Default::default()
    }
    .insert(db)
    .await
    .map_err(AppError::Database)
}

async fn pick_unique_username(db: &DatabaseConnection, base: &str) -> Result<String, AppError> {
    let sanitized: String = base
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '_' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect();
    let sanitized = if sanitized.is_empty() {
        "user".to_string()
    } else {
        sanitized
    };

    if find_by_username_on(db, &sanitized).await?.is_none() {
        return Ok(sanitized);
    }
    for n in 2..=8 {
        let candidate = format!("{sanitized}_{n}");
        if find_by_username_on(db, &candidate).await?.is_none() {
            return Ok(candidate);
        }
    }
    let candidates = (9..1000)
        .map(|n| format!("{sanitized}_{n}"))
        .collect::<Vec<_>>();
    let existing = user::Entity::find()
        .select_only()
        .column(user::Column::Username)
        .filter(user::Column::Username.is_in(candidates.iter().cloned()))
        .into_tuple::<String>()
        .all(db)
        .await
        .map_err(AppError::Database)?
        .into_iter()
        .collect::<HashSet<_>>();
    if let Some(candidate) = candidates
        .into_iter()
        .find(|candidate| !existing.contains(candidate))
    {
        return Ok(candidate);
    }
    Err(AppError::Internal(anyhow::anyhow!(
        "无法为 {sanitized} 分配唯一用户名"
    )))
}
