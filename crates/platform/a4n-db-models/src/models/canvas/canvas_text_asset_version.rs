//! Immutable revisions of project text assets.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "canvas_text_asset_version")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub canvas_text_asset_id: i64,
    pub version: i32,
    #[sea_orm(column_type = "Text")]
    pub content: String,
    pub content_hash: String,
    pub created_at: chrono::NaiveDateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::canvas_text_asset::Entity",
        from = "Column::CanvasTextAssetId",
        to = "super::canvas_text_asset::Column::Id",
        on_delete = "Cascade"
    )]
    Asset,
}

impl ActiveModelBehavior for ActiveModel {}

impl Related<super::canvas_text_asset::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Asset.def()
    }
}
