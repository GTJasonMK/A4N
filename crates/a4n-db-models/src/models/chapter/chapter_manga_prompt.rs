//! 章节漫画提示词实体

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

pub const MANGA_PAGE_PROMPT_TYPE: &str = "page_generation";

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "chapter_manga_prompt")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub chapter_id: i64,
    pub project_id: i64,

    /// 页序号
    pub page_number: i32,

    /// 固定为 page_generation
    pub prompt_type: String,

    /// 图片生成提示词
    #[sea_orm(column_type = "Text")]
    pub image_prompt: String,

    /// 对话/旁白文本
    pub dialogue_text: Option<String>,

    /// 角色出镜列表
    #[sea_orm(column_type = "Json")]
    pub characters_in_frame: Option<serde_json::Value>,

    /// 页面计划及来源、导演计划、参考素材绑定
    #[sea_orm(column_type = "Json")]
    pub layout: Option<serde_json::Value>,

    /// 对应生成图片 ID
    pub generated_image_id: Option<i64>,

    /// 流水线状态：pending / generating / completed / failed / cancelled
    pub status: String,

    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::chapter::Entity",
        from = "Column::ChapterId",
        to = "super::chapter::Column::Id"
    )]
    Chapter,
}

impl ActiveModelBehavior for ActiveModel {}

impl Related<super::chapter::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Chapter.def()
    }
}
