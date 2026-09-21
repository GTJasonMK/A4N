use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "host_package_operation")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub operation_id: String,
    pub host_scope: String,
    pub package_id: String,
    pub action: String,
    pub state: String,
    pub requested_version: Option<String>,
    pub requested_by_user_id: i64,
    pub status_version: i64,
    pub reason_code: Option<String>,
    #[sea_orm(column_type = "Text")]
    pub error: Option<String>,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
