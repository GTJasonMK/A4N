use chrono::Utc;
use sea_orm::*;

use a4n_core::error::AppError;
use a4n_core::models::user::user;

use super::{AuthService, validate_email_identity};

impl AuthService {
    /// OAuth2 登录：按 (provider, provider_id) 查找已有用户；若无则按 email 关联；否则自动注册
    ///
    /// 流程：
    /// 1. 查 (oauth_provider, oauth_subject) 完整匹配 → 命中则更新 last_login_at 并返回
    /// 2. 否则按 email 查找 → 命中则把 oauth_provider/oauth_subject 关联到该账户
    /// 3. 否则新建账户；用户名取自 OAuth username/email 本地部分，遇冲突追加后缀
    ///
    /// password_hash 留空（OAuth2 用户无密码），role 默认 "user"
    pub async fn login_with_oauth2(
        &self,
        provider: &str,
        provider_id: &str,
        email: Option<&str>,
        username_hint: Option<&str>,
        avatar_url: Option<&str>,
    ) -> Result<user::Model, AppError> {
        let provider = normalize_oauth_provider(provider)?;
        let provider_id = normalize_oauth_subject(provider_id)?;
        let now = Utc::now().naive_utc();

        // 1) provider + subject 精确匹配
        if let Some(u) = user::Entity::find()
            .filter(user::Column::OauthProvider.eq(provider.as_str()))
            .filter(user::Column::OauthSubject.eq(provider_id.as_str()))
            .one(self.db.as_ref())
            .await
            .map_err(AppError::Database)?
        {
            if !u.is_active {
                return Err(AppError::Forbidden("账户已被禁用".into()));
            }
            let id = u.id;
            let mut active: user::ActiveModel = u.into();
            active.last_login_at = Set(Some(now));
            if let Some(av) = avatar_url {
                active.avatar_url = Set(Some(av.to_string()));
            }
            active
                .update(self.db.as_ref())
                .await
                .map_err(AppError::Database)?;
            return self.get(id).await;
        }

        let email_addr = required_oauth_email(email)?;

        // 2) 按 email 关联现有账户
        if let Some(u) = self.find_by_email(email_addr).await? {
            if !u.is_active {
                return Err(AppError::Forbidden("账户已被禁用".into()));
            }
            let id = u.id;
            let mut active: user::ActiveModel = u.into();
            active.oauth_provider = Set(Some(provider.clone()));
            active.oauth_subject = Set(Some(provider_id.clone()));
            active.last_login_at = Set(Some(now));
            if let Some(av) = avatar_url {
                active.avatar_url = Set(Some(av.to_string()));
            }
            active.updated_at = Set(now);
            active
                .update(self.db.as_ref())
                .await
                .map_err(AppError::Database)?;
            return self.get(id).await;
        }

        // 3) 自动注册：用户名候选优先 username_hint → email 本地部分
        let base = oauth_username_base(username_hint, email_addr)?;
        let final_username = self.pick_unique_username(&base).await?;

        user::ActiveModel {
            username: Set(final_username),
            email: Set(email_addr.to_string()),
            password_hash: Set(String::new()),
            role: Set("user".to_string()),
            oauth_provider: Set(Some(provider)),
            oauth_subject: Set(Some(provider_id)),
            avatar_url: Set(avatar_url.map(|s| s.to_string())),
            is_active: Set(true),
            created_at: Set(now),
            updated_at: Set(now),
            last_login_at: Set(Some(now)),
            ..Default::default()
        }
        .insert(self.db.as_ref())
        .await
        .map_err(AppError::Database)
    }

    /// 选择一个不冲突的用户名；冲突时追加 `_2` / `_3` …
    async fn pick_unique_username(&self, base: &str) -> Result<String, AppError> {
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

        if self.find_by_username(&sanitized).await?.is_none() {
            return Ok(sanitized);
        }
        for n in 2..1000 {
            let candidate = format!("{sanitized}_{n}");
            if self.find_by_username(&candidate).await?.is_none() {
                return Ok(candidate);
            }
        }
        Err(AppError::Internal(anyhow::anyhow!(
            "无法为 {sanitized} 分配唯一用户名"
        )))
    }
}

fn normalize_oauth_provider(provider: &str) -> Result<String, AppError> {
    let provider = provider.trim();
    match provider {
        "" => Err(AppError::InvalidArgument("OAuth2 提供商不能为空".into())),
        "github" | "google" => Ok(provider.to_string()),
        _ => Err(AppError::InvalidArgument("不支持的 OAuth2 提供商".into())),
    }
}

fn normalize_oauth_subject(provider_id: &str) -> Result<String, AppError> {
    let provider_id = provider_id.trim();
    if provider_id.is_empty() {
        return Err(AppError::InvalidArgument("OAuth2 用户标识不能为空".into()));
    }
    Ok(provider_id.to_string())
}

fn required_oauth_email(email: Option<&str>) -> Result<&str, AppError> {
    let email = email
        .map(str::trim)
        .filter(|email| !email.is_empty())
        .ok_or_else(|| AppError::InvalidArgument("OAuth 用户信息缺少 email".into()))?;
    validate_email_identity(email)
        .map_err(|_| AppError::InvalidArgument("OAuth email 格式无效".into()))?;
    Ok(email)
}

fn oauth_username_base(username_hint: Option<&str>, email: &str) -> Result<String, AppError> {
    if let Some(username) = username_hint
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        return Ok(username.to_string());
    }
    let local = email
        .split_once('@')
        .map(|(local, _)| local.trim())
        .filter(|local| !local.is_empty())
        .ok_or_else(|| AppError::InvalidArgument("OAuth email 缺少用户名部分".into()))?;
    Ok(local.to_string())
}
