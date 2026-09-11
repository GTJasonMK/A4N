use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "host_package_state")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub host_scope: String,
    #[sea_orm(primary_key, auto_increment = false)]
    pub package_id: String,
    pub installation_state: String,
    pub host_state: String,
    pub installed_version: Option<String>,
    pub status_version: i64,
    pub updated_at: chrono::NaiveDateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
