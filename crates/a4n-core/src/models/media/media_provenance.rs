//! 媒体来源实体

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "media_provenance")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub media_asset_id: i64,
    pub generation_spec_id: Option<i64>,
    pub prompt_artifact_id: Option<i64>,
    pub provider_job_id: Option<String>,
    pub source_media_asset_id: Option<i64>,
    #[sea_orm(column_type = "Text")]
    pub source_external_path: Option<String>,
    pub imported_from_user: bool,
    #[sea_orm(column_type = "Json")]
    pub request_snapshot_json: Option<serde_json::Value>,
    pub created_at: chrono::NaiveDateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
