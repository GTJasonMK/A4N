//! 生成意图语义实体引用实体

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "generation_spec_entity_ref")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub generation_spec_id: i64,
    #[sea_orm(primary_key, auto_increment = false)]
    pub entity_kind: String,
    #[sea_orm(primary_key, auto_increment = false)]
    pub entity_id: i64,
    #[sea_orm(primary_key, auto_increment = false)]
    pub role: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
