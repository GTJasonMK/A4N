//! 生成意图媒体引用实体

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "generation_spec_media_ref")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub generation_spec_id: i64,
    #[sea_orm(primary_key, auto_increment = false)]
    pub media_asset_id: i64,
    #[sea_orm(primary_key, auto_increment = false)]
    pub role: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
