//! 场景档案实体。

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "scene_profile")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub project_id: i64,

    pub ownership_kind: String,

    pub name: String,
    pub normalized_name: String,

    #[sea_orm(column_type = "Json")]
    pub aliases_json: serde_json::Value,

    pub scene_type: String,
    pub importance: String,
    pub parent_scene_id: Option<i64>,

    #[sea_orm(column_type = "Text")]
    pub base_description: String,

    #[sea_orm(column_type = "Text")]
    pub visual_description: String,

    #[sea_orm(column_type = "Text")]
    pub spatial_layout: String,

    #[sea_orm(column_type = "Text")]
    pub atmosphere: String,

    #[sea_orm(column_type = "Json")]
    pub entry_points_json: serde_json::Value,

    #[sea_orm(column_type = "Json")]
    pub rules_json: serde_json::Value,

    #[sea_orm(column_type = "Json")]
    pub current_state_json: serde_json::Value,

    pub first_seen_chapter: i32,
    pub last_seen_chapter: i32,

    #[sea_orm(column_type = "Json")]
    pub evidence_json: serde_json::Value,

    #[sea_orm(column_type = "Text")]
    pub image_prompt_seed: String,

    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::super::project::novel_project::Entity",
        from = "Column::ProjectId",
        to = "super::super::project::novel_project::Column::Id"
    )]
    NovelProject,
}

impl ActiveModelBehavior for ActiveModel {}

impl Related<super::super::project::novel_project::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::NovelProject.def()
    }
}

#[cfg(test)]
mod tests {
    use super::Model;
    use serde_json::json;

    #[test]
    fn ownership_kind_is_required_when_deserializing() {
        let error = serde_json::from_value::<Model>(json!({
            "id": 1,
            "project_id": 2,
            "name": "旧书房",
            "normalized_name": "旧书房",
            "aliases_json": [],
            "scene_type": "interior",
            "importance": "primary",
            "parent_scene_id": null,
            "base_description": "旧书房",
            "visual_description": "落满灰尘",
            "spatial_layout": "书架环绕",
            "atmosphere": "安静",
            "entry_points_json": [],
            "rules_json": [],
            "current_state_json": {},
            "first_seen_chapter": 1,
            "last_seen_chapter": 1,
            "evidence_json": [],
            "image_prompt_seed": "旧书房",
            "created_at": "2026-08-19T00:00:00",
            "updated_at": "2026-08-19T00:00:00"
        }))
        .expect_err("missing ownership_kind must fail");

        assert!(error.to_string().contains("ownership_kind"));
    }
}
