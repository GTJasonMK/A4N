//! 模型生成运行目标实体。

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "generation_run_target")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub generation_run_id: i64,
    pub target_kind: String,
    pub target_table: String,
    pub target_id: i64,
    pub role: String,
    pub created_at: chrono::NaiveDateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::generation_run::Entity",
        from = "Column::GenerationRunId",
        to = "super::generation_run::Column::Id"
    )]
    GenerationRun,
}

impl ActiveModelBehavior for ActiveModel {}

impl Related<super::generation_run::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::GenerationRun.def()
    }
}
