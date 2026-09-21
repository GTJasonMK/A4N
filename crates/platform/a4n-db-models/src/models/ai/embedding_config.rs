//! 嵌入模型配置实体

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "embedding_config")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,

    pub name: String,

    /// 后端：api / fastembed / local_hash
    pub backend: String,

    pub base_url: Option<String>,
    pub api_key_encrypted: Option<String>,

    pub model: String,
    pub dimension: i32,
    pub batch_size: i32,

    /// 本地模型路径
    pub local_model_path: Option<String>,

    #[sea_orm(column_type = "JsonBinary")]
    pub pricing_json: serde_json::Value,

    pub is_default: bool,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
