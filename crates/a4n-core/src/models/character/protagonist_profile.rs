//! 主角档案实体

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "protagonist_profile")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub project_id: i64,

    pub name: String,
    pub gender: Option<String>,
    pub age: Option<String>,

    /// 基础属性（JSON）
    #[sea_orm(column_type = "Json")]
    pub base_attributes: serde_json::Value,

    /// 当前属性状态（事件溯源折叠结果）
    #[sea_orm(column_type = "Json")]
    pub current_state: serde_json::Value,

    /// 角色弧光定义
    #[sea_orm(column_type = "Json")]
    pub character_arc: Option<serde_json::Value>,

    /// 角色声音指纹
    #[sea_orm(column_type = "Json")]
    pub voice_fingerprint: Option<serde_json::Value>,

    /// 外观描述
    #[sea_orm(column_type = "Text")]
    pub appearance: Option<String>,

    /// 背景故事
    #[sea_orm(column_type = "Text")]
    pub background: Option<String>,

    /// 人格描述
    #[sea_orm(column_type = "Text")]
    pub personality: Option<String>,

    /// 档案已经同步到的章节号
    pub last_synced_chapter: i32,

    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::super::project::novel_project::Entity",
        from = "Column::ProjectId",
        to = "super::super::project::novel_project::Column::Id"
    )]
    NovelProject,
    #[sea_orm(has_many = "super::attribute_event::Entity")]
    AttributeEvent,
}

impl ActiveModelBehavior for ActiveModel {}

impl Model {
    pub fn first_seen_chapter(&self) -> Option<i32> {
        let explicit = self
            .base_attributes
            .get("explicit_attributes")
            .and_then(serde_json::Value::as_object)?;
        let first_seen = explicit.get("first_seen_chapter")?.as_i64()?;
        i32::try_from(first_seen)
            .ok()
            .filter(|chapter| *chapter > 0)
    }

    pub fn is_text_discovered(&self) -> bool {
        self.first_seen_chapter().is_some()
    }
}

impl Related<super::super::project::novel_project::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::NovelProject.def()
    }
}

impl Related<super::attribute_event::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::AttributeEvent.def()
    }
}
