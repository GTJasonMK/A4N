//! 主题配置实体

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "theme_config")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub user_id: Option<i64>,

    /// 主题名称
    pub name: String,

    /// 主题模式：light / dark
    pub mode: String,

    /// 版本化、受控的主题文档。
    #[sea_orm(column_type = "Json")]
    pub theme_document: serde_json::Value,

    /// 是否是系统默认
    pub is_system_default: bool,

    /// 是否当前激活
    pub is_active: bool,

    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
