//! 生成意图文本锚点引用实体

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "generation_spec_text_anchor")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub generation_spec_id: i64,
    #[sea_orm(primary_key, auto_increment = false)]
    pub text_anchor_id: i64,
    #[sea_orm(primary_key, auto_increment = false)]
    pub role: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
