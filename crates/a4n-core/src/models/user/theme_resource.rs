//! 用户主题背景资源元数据。

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "theme_resource")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub theme_id: i64,
    /// Stable logical key used by theme documents and theme packages.
    pub resource_key: String,
    pub file_name: String,
    pub mime_type: String,
    pub file_path: String,
    pub sha256: String,
    pub byte_size: i64,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
