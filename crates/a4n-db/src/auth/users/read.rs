use a4n_core::error::AppError;
use a4n_core::models::user::user;
use a4n_domain::auth::UserSummary;
use chrono::NaiveDateTime;
use sea_orm::{
    ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait, FromQueryResult, PaginatorTrait,
    QueryFilter, QueryOrder, QuerySelect,
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

impl CurrentUserSummaryRow {
    fn into_summary(self) -> UserSummary {
        UserSummary {
            id: self.id,
            username: self.username,
            email: self.email,
            role: self.role,
            oauth_provider: self.oauth_provider,
            oauth_subject: self.oauth_subject,
            avatar_url: self.avatar_url,
            page_cli_enabled: self.page_cli_enabled,
            is_active: self.is_active,
            created_at: Some(self.created_at.to_string()),
            updated_at: Some(self.updated_at.to_string()),
            last_login_at: self.last_login_at.map(|value| value.to_string()),
        }
    }
}

pub async fn user_by_id(db: &DatabaseConnection, user_id: i64) -> Result<user::Model, AppError> {
    user::Entity::find_by_id(user_id)
        .one(db)
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| AppError::NotFound(format!("用户不存在: {user_id}")))
}

pub async fn current_user(db: &DatabaseConnection, user_id: i64) -> Result<user::Model, AppError> {
    active_user(db, user_id).await
}

pub async fn current_user_summary(
    db: &DatabaseConnection,
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
        .map_err(AppError::Database)?
        .ok_or_else(|| AppError::NotFound(format!("用户不存在: {user_id}")))?;
    if !user.is_active {
        return Err(AppError::Forbidden("账户已被禁用".into()));
    }
    Ok(user.into_summary())
}

pub async fn count_users(db: &DatabaseConnection) -> Result<u64, AppError> {
    user::Entity::find()
        .count(db)
        .await
        .map_err(AppError::Database)
}

pub async fn list_users(db: &DatabaseConnection) -> Result<Vec<UserSummary>, AppError> {
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
        .map_err(AppError::Database)
        .map(|users| {
            users
                .into_iter()
                .map(CurrentUserSummaryRow::into_summary)
                .collect()
        })
}

pub(in crate::auth) async fn active_user(
    db: &DatabaseConnection,
    user_id: i64,
) -> Result<user::Model, AppError> {
    let user = user_by_id(db, user_id).await?;
    if !user.is_active {
        return Err(AppError::Forbidden("账户已被禁用".into()));
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
        .map_err(AppError::Database)
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
        .map_err(AppError::Database)
}
