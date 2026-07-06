//! 媒体用途绑定实体

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "media_binding")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub media_asset_id: i64,
    pub target_kind: String,
    pub target_id: i64,
    pub role: String,
    pub created_at: chrono::NaiveDateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
