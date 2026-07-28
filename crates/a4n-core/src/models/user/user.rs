//! 用户实体

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "user")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,

    pub username: String,
    pub email: String,

    /// 密码哈希（argon2）
    pub password_hash: String,

    /// 角色：admin / user / read_only
    pub role: String,

    /// OAuth2 提供商
    pub oauth_provider: Option<String>,
    pub oauth_subject: Option<String>,

    /// 头像 URL
    pub avatar_url: Option<String>,

    pub is_active: bool,
    /// 内部破坏性操作门禁：active / disabling / disabled
    #[serde(skip)]
    pub lifecycle_gate: String,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
    pub last_login_at: Option<chrono::NaiveDateTime>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
