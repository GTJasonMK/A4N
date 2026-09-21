//! 图片生成配置实体

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "image_gen_config")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,

    pub name: String,
    pub backend: String,
    pub base_url: Option<String>,
    pub api_key_encrypted: Option<String>,

    pub model: String,
    pub max_concurrent: i32,
    #[sea_orm(column_type = "JsonBinary")]
    pub supported_sizes_json: serde_json::Value,
    #[sea_orm(column_type = "JsonBinary")]
    pub pricing_json: serde_json::Value,
    pub is_default: bool,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
