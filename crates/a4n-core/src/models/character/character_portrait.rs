//! 角色立绘实体

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "character_portrait")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub profile_id: i64,
    pub project_id: i64,

    /// 立绘类型：full_body / half_body / face / chibi
    pub portrait_type: String,

    /// 图片路径
    pub image_path: String,

    /// 生成提示词
    #[sea_orm(column_type = "Text")]
    pub prompt: String,

    /// 表情/姿态
    pub expression: Option<String>,
    pub pose: Option<String>,

    /// 是否为主立绘
    pub is_primary: bool,

    pub created_at: chrono::NaiveDateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
