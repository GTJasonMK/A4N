//! Immutable migration evidence. The old/new IDs describe the original
//! migration namespace; only project_id is remapped when a package is imported.
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "media_authoring_migration_receipt")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub kind: String,
    #[sea_orm(primary_key, auto_increment = false)]
    pub old_id: i64,
    #[sea_orm(primary_key, auto_increment = false)]
    pub new_id: i64,
    #[sea_orm(primary_key, auto_increment = false)]
    pub project_id: i64,
    #[sea_orm(column_type = "JsonBinary")]
    pub before_json: serde_json::Value,
    pub after_sha256: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
impl ActiveModelBehavior for ActiveModel {}
