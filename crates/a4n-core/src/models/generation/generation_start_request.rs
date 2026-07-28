//! 生成启动请求的稳定幂等映射。

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "generation_start_request")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub requested_by_user_id: i64,
    #[sea_orm(primary_key, auto_increment = false)]
    pub start_request_id: Uuid,
    pub task_id: String,
    pub input_fingerprint_sha256: String,
    pub created_at: chrono::NaiveDateTime,
    pub last_seen_at: chrono::NaiveDateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
