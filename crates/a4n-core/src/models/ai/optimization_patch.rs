use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "optimization_patch")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub patch_set_id: i64,
    pub patch_id: String,
    pub issue_id: Option<String>,
    pub operation: String,

    #[sea_orm(column_type = "Text")]
    pub old_text: String,

    #[sea_orm(column_type = "Text")]
    pub new_text: String,

    #[sea_orm(column_type = "Text")]
    pub reason: String,

    #[sea_orm(column_type = "Text")]
    pub risk: Option<String>,

    pub decision: String,
    pub apply_status: String,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::optimization_patch_set::Entity",
        from = "Column::PatchSetId",
        to = "super::optimization_patch_set::Column::Id"
    )]
    PatchSet,
}

impl ActiveModelBehavior for ActiveModel {}

impl Related<super::optimization_patch_set::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::PatchSet.def()
    }
}
