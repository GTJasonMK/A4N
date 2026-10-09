//! 漫画流水线的单阶段最新产物。

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "manga_pipeline_artifact")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub project_id: i64,
    pub chapter_id: i64,
    pub stage: String,
    /// 0 表示章节级产物；正数表示漫画页码。
    pub page_number: i32,
    pub input_fingerprint: String,
    pub output_fingerprint: String,
    #[sea_orm(column_type = "Json")]
    pub payload: serde_json::Value,
    pub stale: bool,
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
    Project,
}

impl ActiveModelBehavior for ActiveModel {}

impl Related<super::super::project::novel_project::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Project.def()
    }
}
