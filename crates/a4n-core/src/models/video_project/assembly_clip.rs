//! 成片时间线片段实体

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "assembly_clip")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub timeline_id: i64,
    pub media_asset_id: i64,
    pub order_index: i32,
    pub trim_start_secs: f32,
    pub trim_end_secs: f32,
    pub volume: f32,
    pub transition_kind: String,
    pub source_segment_id: Option<i64>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
