//! 蓝图角色实体

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "blueprint_character")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub project_id: i64,

    pub name: String,

    /// 角色类型：protagonist / antagonist / supporting / minor
    pub character_type: String,

    pub gender: Option<String>,
    pub age: Option<String>,
    pub appearance: Option<String>,
    pub personality: Option<String>,
    pub background: Option<String>,
    pub motivation: Option<String>,
    pub arc_summary: Option<String>,

    /// 与主角的关系描述
    pub relationship_to_protagonist: Option<String>,

    /// 角色弧光阶段
    #[sea_orm(column_type = "Json")]
    pub arc_stages: Option<serde_json::Value>,

    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::novel_blueprint::Entity",
        from = "Column::ProjectId",
        to = "super::novel_blueprint::Column::ProjectId"
    )]
    NovelBlueprint,
}

impl ActiveModelBehavior for ActiveModel {}

impl Related<super::novel_blueprint::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::NovelBlueprint.def()
    }
}
