//! 小说项目实体

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "novel_project")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub user_id: i64,

    pub title: String,
    #[sea_orm(column_type = "Text")]
    pub description: String,

    /// 项目类型：novel_long / novel_short / script / prose
    pub project_type: String,

    /// 项目族类型：text / comic / video / galgame
    pub project_kind: String,

    /// 来源模式：self / derived / freeform
    pub source_mode: String,

    /// 派生项目指向文本根项目；文本项目为空
    pub root_project_id: Option<i64>,

    /// 派生项目绑定的源文本快照
    pub source_snapshot_id: Option<i64>,

    /// current / root_updated / conflict / detached
    pub source_sync_status: String,

    /// 状态：draft / blueprint_ready / part_outlines_ready / chapter_outlines_ready / writing / completed
    pub status: String,

    /// 内部破坏性操作门禁：active / deleting / resetting
    #[serde(skip)]
    pub lifecycle_gate: String,

    /// 创作进度
    pub target_chapters: i32,
    pub current_chapter: i32,
    pub total_chapters: Option<i32>,

    /// 分部大纲配置
    pub chapters_per_part: Option<i32>,

    /// 头像 SVG
    #[sea_orm(column_type = "Text")]
    pub avatar_svg: Option<String>,
    pub avatar_animal: Option<String>,

    /// 大型 JSON 字段
    #[sea_orm(column_type = "Text")]
    pub raw_blueprint: Option<String>,
    #[sea_orm(column_type = "Text")]
    pub import_analysis_progress: Option<String>,
    #[sea_orm(column_type = "Text")]
    pub export_config: Option<String>,

    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::novel_blueprint::Entity")]
    NovelBlueprint,
    #[sea_orm(has_many = "super::super::chapter::chapter::Entity")]
    Chapter,
    #[sea_orm(has_many = "super::super::user::conversation::Entity")]
    Conversation,
}

impl ActiveModelBehavior for ActiveModel {}

impl Related<super::novel_blueprint::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::NovelBlueprint.def()
    }
}

impl Related<super::super::chapter::chapter::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Chapter.def()
    }
}

impl Related<super::super::user::conversation::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Conversation.def()
    }
}
