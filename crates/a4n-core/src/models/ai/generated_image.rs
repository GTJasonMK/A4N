//! 生成图片实体

use sea_orm::{FromQueryResult, entity::prelude::*};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "generated_image")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub project_id: i64,

    /// 图片类型：portrait / manga_page / scene / cover / illustration
    pub image_type: String,

    /// 文件路径
    pub file_path: String,

    /// 生成提示词
    #[sea_orm(column_type = "Text")]
    pub prompt: String,

    /// 使用的模型
    pub model: String,

    /// 图片宽度
    pub width: i32,
    /// 图片高度
    pub height: i32,

    /// 文件大小（字节）
    pub file_size: Option<i64>,

    /// AI 评分
    pub ai_score: Option<f32>,

    /// 是否被用户收藏
    pub is_favorite: bool,

    /// 关联章节
    pub chapter_id: Option<i64>,

    /// 关联漫画分镜/页面标识
    pub panel_id: Option<String>,

    /// 生成元数据
    #[sea_orm(column_type = "Json")]
    pub metadata_json: Option<serde_json::Value>,

    pub created_at: chrono::NaiveDateTime,
}

/// 只用于视频提示词引用匹配的轻量图片记录。
#[derive(Clone, Debug, PartialEq, FromQueryResult)]
pub struct Reference {
    pub id: i64,
    pub image_type: String,
    pub metadata_json: Option<serde_json::Value>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
