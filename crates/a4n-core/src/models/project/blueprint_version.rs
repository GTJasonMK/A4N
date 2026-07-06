//! 蓝图版本历史实体【新增】

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "blueprint_version")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub project_id: i64,

    /// 版本号：1.0 / 1.1 / 2.0
    pub version: String,
    pub change_description: Option<String>,

    /// 完整蓝图快照
    #[sea_orm(column_type = "Json")]
    pub blueprint_data: serde_json::Value,

    /// 分阶段快照
    #[sea_orm(column_type = "Json")]
    pub world_setting_snapshot: Option<serde_json::Value>,
    #[sea_orm(column_type = "Json")]
    pub characters_snapshot: Option<serde_json::Value>,
    #[sea_orm(column_type = "Json")]
    pub relationships_snapshot: Option<serde_json::Value>,
    #[sea_orm(column_type = "Text")]
    pub synopsis_snapshot: Option<String>,

    pub created_at: chrono::NaiveDateTime,
    pub created_by: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::novel_project::Entity",
        from = "Column::ProjectId",
        to = "super::novel_project::Column::Id"
    )]
    NovelProject,
}

impl ActiveModelBehavior for ActiveModel {}

impl Related<super::novel_project::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::NovelProject.def()
    }
}
