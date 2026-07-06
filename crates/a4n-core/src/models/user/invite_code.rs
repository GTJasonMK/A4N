//! 注册邀请码实体

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "invite_code")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,

    pub code: String,
    pub created_by_user_id: Option<i64>,
    pub used_by_user_id: Option<i64>,
    pub created_at: chrono::NaiveDateTime,
    pub used_at: Option<chrono::NaiveDateTime>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::CreatedByUserId",
        to = "super::user::Column::Id"
    )]
    CreatedByUser,
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::UsedByUserId",
        to = "super::user::Column::Id"
    )]
    UsedByUser,
}

impl ActiveModelBehavior for ActiveModel {}
