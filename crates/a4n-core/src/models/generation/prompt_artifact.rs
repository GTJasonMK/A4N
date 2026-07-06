//! 提示词产物实体

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "prompt_artifact")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub generation_spec_id: i64,
    pub prompt_kind: String,
    #[sea_orm(column_type = "Text")]
    pub prompt_text: String,
    pub renderer_version: String,
    pub edited_by_user: bool,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
