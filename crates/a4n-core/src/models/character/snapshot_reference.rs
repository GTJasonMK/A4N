//! 快照引用实体【新增】—— 内容寻址存储

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "snapshot_reference")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub profile_id: i64,
    pub chapter_number: i32,

    /// SHA256 哈希
    pub snapshot_hash: String,

    /// 引用计数
    pub reference_count: i32,

    /// 快照内容
    #[sea_orm(column_type = "Json")]
    pub data: serde_json::Value,

    pub created_at: chrono::NaiveDateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
