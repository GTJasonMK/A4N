use a4n_db_models::models::user::user;
use a4n_domain::auth::{UserAuthorizationRecord, UserSummary};
use a4n_error::AppError;
use chrono::NaiveDateTime;
use sea_orm::{
    ColumnTrait, ConnectionTrait, EntityTrait, FromQueryResult, PaginatorTrait, QueryFilter,
    QueryOrder, QuerySelect,
};

#[derive(Debug, FromQueryResult)]
struct CurrentUserSummaryRow {
    id: i64,
    username: String,
    email: String,
    role: String,
    oauth_provider: Option<String>,
    oauth_subject: Option<String>,
    avatar_url: Option<String>,
    page_cli_enabled: bool,
    is_active: bool,
    created_at: NaiveDateTime,
    updated_at: NaiveDateTime,
    last_login_at: Option<NaiveDateTime>,
}

#[derive(Debug, FromQueryResult)]
struct UserAuthorizationRow {
    id: i64,
    role: String,
    is_active: bool,
}

impl CurrentUserSummaryRow {
    fn into_summary(self) -> UserSummary {
        user_summary_from_fields(
            self.id,
            self.username,
            self.email,
            self.role,
            self.oauth_provider,
            self.oauth_subject,
            self.avatar_url,
            self.page_cli_enabled,
            self.is_active,
            self.created_at,
            self.updated_at,
            self.last_login_at,
        )
    }
}

pub(in crate::auth) fn user_summary_from_model(user: &user::Model) -> UserSummary {
    user_summary_from_fields(
        user.id,
        user.username.clone(),
        user.email.clone(),
        user.role.clone(),
        user.oauth_provider.clone(),
        user.oauth_subject.clone(),
        user.avatar_url.clone(),
        user.page_cli_enabled,
        user.is_active,
        user.created_at,
        user.updated_at,
        user.last_login_at,
    )
}

fn user_summary_from_fields(
    id: i64,
    username: String,
    email: String,
    role: String,
    oauth_provider: Option<String>,
    oauth_subject: Option<String>,
    avatar_url: Option<String>,
    page_cli_enabled: bool,
    is_active: bool,
    created_at: NaiveDateTime,
    updated_at: NaiveDateTime,
    last_login_at: Option<NaiveDateTime>,
) -> UserSummary {
    UserSummary {
        id,
        username,
        email,
        role,
        oauth_provider,
        oauth_subject,
        avatar_url,
        page_cli_enabled,
        is_active,
        created_at: Some(created_at.and_utc().to_rfc3339()),
        updated_at: Some(updated_at.and_utc().to_rfc3339()),
        last_login_at: last_login_at.map(|value| value.and_utc().to_rfc3339()),
    }
}

pub async fn user_by_id(
    db: &impl crate::DbExecutor,
    user_id: i64,
) -> Result<user::Model, AppError> {
    user::Entity::find_by_id(user_id)
        .one(db)
        .await
        .map_err(AppError::from)?
        .ok_or_else(|| AppError::NotFound(format!("用户不存在: {user_id}")))
}

pub async fn user_authorization_by_id(
    db: &impl crate::DbExecutor,
    user_id: i64,
) -> Result<UserAuthorizationRecord, AppError> {
    let user = user::Entity::find_by_id(user_id)
        .select_only()
        .column(user::Column::Id)
        .column(user::Column::Role)
        .column(user::Column::IsActive)
        .into_model::<UserAuthorizationRow>()
        .one(db)
        .await
        .map_err(AppError::from)?
        .ok_or_else(|| AppError::NotFound(format!("用户不存在: {user_id}")))?;
    Ok(UserAuthorizationRecord {
        id: user.id,
        role: user.role,
        is_active: user.is_active,
    })
}

#[cfg(test)]
pub async fn current_user(
    db: &impl crate::DbExecutor,
    user_id: i64,
) -> Result<user::Model, AppError> {
    active_user(db, user_id).await
}

pub async fn current_user_summary(
    db: &impl crate::DbExecutor,
    user_id: i64,
) -> Result<UserSummary, AppError> {
    let summary = user_summary_by_id(db, user_id).await?;
    if !summary.is_active {
        return Err(AppError::Unauthorized("账户已被禁用".into()));
    }
    Ok(summary)
}

pub async fn user_summary_by_id(
    db: &impl crate::DbExecutor,
    user_id: i64,
) -> Result<UserSummary, AppError> {
    let user = user::Entity::find_by_id(user_id)
        .select_only()
        .column(user::Column::Id)
        .column(user::Column::Username)
        .column(user::Column::Email)
        .column(user::Column::Role)
        .column(user::Column::OauthProvider)
        .column(user::Column::OauthSubject)
        .column(user::Column::AvatarUrl)
        .column(user::Column::PageCliEnabled)
        .column(user::Column::IsActive)
        .column(user::Column::CreatedAt)
        .column(user::Column::UpdatedAt)
        .column(user::Column::LastLoginAt)
        .into_model::<CurrentUserSummaryRow>()
        .one(db)
        .await
        .map_err(AppError::from)?
        .ok_or_else(|| AppError::NotFound(format!("用户不存在: {user_id}")))?;
    Ok(user.into_summary())
}

pub async fn count_users(db: &impl crate::DbExecutor) -> Result<u64, AppError> {
    user::Entity::find().count(db).await.map_err(AppError::from)
}

pub async fn list_users(db: &impl crate::DbExecutor) -> Result<Vec<UserSummary>, AppError> {
    user::Entity::find()
        .select_only()
        .column(user::Column::Id)
        .column(user::Column::Username)
        .column(user::Column::Email)
        .column(user::Column::Role)
        .column(user::Column::OauthProvider)
        .column(user::Column::OauthSubject)
        .column(user::Column::AvatarUrl)
        .column(user::Column::PageCliEnabled)
        .column(user::Column::IsActive)
        .column(user::Column::CreatedAt)
        .column(user::Column::UpdatedAt)
        .column(user::Column::LastLoginAt)
        .order_by_asc(user::Column::Id)
        .into_model::<CurrentUserSummaryRow>()
        .all(db)
        .await
        .map_err(AppError::from)
        .map(|users| {
            users
                .into_iter()
                .map(CurrentUserSummaryRow::into_summary)
                .collect()
        })
}

#[cfg(test)]
mod tests {
    use super::user_summary_from_model;
    use a4n_db_models::models::user::user;

    #[test]
    fn user_summary_does_not_expose_database_secrets() {
        let now = Default::default();
        let user = user::Model {
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

        let serialized = serde_json::to_value(user_summary_from_model(&user)).unwrap();
        assert_eq!(serialized["id"], 12);
        assert_eq!(serialized["username"], "reader");
        assert!(serialized.get("password_hash").is_none());
        assert!(serialized.get("lifecycle_gate").is_none());
        assert!(!serialized.to_string().contains("password-hash-secret"));
    }
}

pub(in crate::auth) async fn active_user(
    db: &impl crate::DbExecutor,
    user_id: i64,
) -> Result<user::Model, AppError> {
    let user = user_by_id(db, user_id).await?;
    if !user.is_active {
        return Err(AppError::Unauthorized("账户已被禁用".into()));
    }
    Ok(user)
}

pub(in crate::auth) async fn find_by_username_on<C>(
    db: &C,
    username: &str,
) -> Result<Option<user::Model>, AppError>
where
    C: ConnectionTrait,
{
    user::Entity::find()
        .filter(user::Column::Username.eq(username))
        .one(db)
        .await
        .map_err(AppError::from)
}

pub(in crate::auth) async fn find_by_email_on<C>(
    db: &C,
    email: &str,
) -> Result<Option<user::Model>, AppError>
where
    C: ConnectionTrait,
{
    user::Entity::find()
        .filter(user::Column::Email.eq(email))
        .one(db)
        .await
        .map_err(AppError::from)
}
