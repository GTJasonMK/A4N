//! 章节场景实体【新增】—— 支持场景级生成和独立修改

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "chapter_scene")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub chapter_id: i64,

    /// 场景在章节内的序号（1-based）
    pub scene_number: i32,

    /// 场景标题/名称
    pub title: Option<String>,

    /// 场景正文
    #[sea_orm(column_type = "Text")]
    pub content: String,

    pub word_count: i32,

    /// 场景摘要
    #[sea_orm(column_type = "Text")]
    pub summary: Option<String>,

    /// 场景类型：opening / development / climax / transition / closing
    pub scene_type: String,

    /// 场景涉及的角色
    #[sea_orm(column_type = "Json")]
    pub characters_present: Option<serde_json::Value>,

    /// 场景地点
    pub location: Option<String>,

    /// 时间点（相对于故事时间线）
    pub timeline_position: Option<String>,

    /// 前一场景的过渡文本
    #[sea_orm(column_type = "Text")]
    pub transition_from_previous: Option<String>,

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
