//! 生成意图实体

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

use crate::source_spans::source_text_hash;

pub const SOURCE_CONTEXT_FREEFORM: &str = "freeform";

pub fn freeform_source_version_hash(params_json: &serde_json::Value) -> Option<String> {
    params_json
        .get("source_material")
        .filter(|value| value.is_object())
        .map(|value| source_text_hash(&value.to_string()))
}

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "generation_spec")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub project_id: i64,
    pub root_project_id: i64,
    pub source_context_kind: Option<String>,
    pub source_snapshot_id: Option<i64>,
    pub source_chapter_id: Option<i64>,
    pub source_snapshot_chapter_id: Option<i64>,
    pub canvas_text_asset_id: Option<i64>,
    pub target_kind: String,
    pub target_entity_kind: String,
    pub target_entity_id: Option<i64>,
    pub status: String,
    pub source_version_hash: String,
    #[sea_orm(column_type = "Json")]
    pub params_json: serde_json::Value,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

#[cfg(test)]
mod tests {
    use super::freeform_source_version_hash;
    use serde_json::json;

    #[test]
    fn freeform_source_version_hash_requires_object_and_is_stable() {
        assert_eq!(
            freeform_source_version_hash(&json!({"source_material":{"a":1,"b":2}})),
            freeform_source_version_hash(&json!({"source_material":{"b":2,"a":1}}))
        );
        assert_eq!(
            freeform_source_version_hash(&json!({"source_material":"legacy"})),
            None
        );
    }
}
