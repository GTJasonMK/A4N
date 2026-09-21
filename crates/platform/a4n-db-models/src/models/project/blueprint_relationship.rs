//! 蓝图人物关系实体

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "blueprint_relationship")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub project_id: i64,

    /// 关系类型：family / friend / enemy / lover / rival / mentor / other
    pub relationship_type: String,

    /// 角色 A 名称
    pub character_a: String,
    /// 角色 B 名称
    pub character_b: String,

    pub description: Option<String>,

    /// 关系强度（1~10）
    pub strength: Option<i32>,

    /// 关系变化弧线
    #[sea_orm(column_type = "Json")]
    pub evolution: Option<serde_json::Value>,

    pub created_at: chrono::NaiveDateTime,
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
