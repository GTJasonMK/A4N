//! 场景出场记录实体。

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "scene_occurrence")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub project_id: i64,
    pub scene_profile_id: i64,
    pub chapter_id: i64,
    pub chapter_number: i32,
    pub occurrence_order: i32,

    pub timeline_position: Option<String>,

    #[sea_orm(column_type = "Text")]
    pub summary: String,

    pub mood: Option<String>,
    pub weather_or_lighting: Option<String>,

    #[sea_orm(column_type = "Json")]
    pub source_span_ids_json: serde_json::Value,

    #[sea_orm(column_type = "Json")]
    pub evidence_json: serde_json::Value,

    #[sea_orm(column_type = "Json")]
    pub characters_json: serde_json::Value,

    #[sea_orm(column_type = "Json")]
    pub objects_json: serde_json::Value,

    #[sea_orm(column_type = "Json")]
    pub key_events_json: serde_json::Value,

    #[sea_orm(column_type = "Json")]
    pub entry_state_json: serde_json::Value,

    #[sea_orm(column_type = "Json")]
    pub exit_state_json: serde_json::Value,

    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::scene_profile::Entity",
        from = "Column::SceneProfileId",
        to = "super::scene_profile::Column::Id"
    )]
    SceneProfile,
    #[sea_orm(
        belongs_to = "super::super::chapter::chapter::Entity",
        from = "Column::ChapterId",
        to = "super::super::chapter::chapter::Column::Id"
    )]
    Chapter,
}

impl ActiveModelBehavior for ActiveModel {}

impl Model {
    pub fn derived_state_json(&self) -> serde_json::Value {
        serde_json::json!({
            "chapter_number": self.chapter_number,
            "occurrence_order": self.occurrence_order,
            "summary": self.summary,
            "mood": self.mood,
            "weather_or_lighting": self.weather_or_lighting,
            "entry_state": self.entry_state_json,
            "exit_state": self.exit_state_json,
            "characters": self.characters_json,
            "objects": self.objects_json,
            "key_events": self.key_events_json,
        })
    }
}

impl Related<super::scene_profile::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::SceneProfile.def()
    }
}

impl Related<super::super::chapter::chapter::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Chapter.def()
    }
}

#[cfg(test)]
mod tests {
    use super::Model;
    use chrono::NaiveDateTime;
    use serde_json::json;

    #[test]
    fn derived_state_keeps_the_selected_occurrence_identity() {
        let occurrence = Model {
            id: 1,
            project_id: 2,
            scene_profile_id: 3,
            chapter_id: 4,
            chapter_number: 5,
            occurrence_order: 6,
            timeline_position: None,
            summary: "雨夜会面".to_string(),
            mood: Some("紧张".to_string()),
            weather_or_lighting: None,
            source_span_ids_json: json!([]),
            evidence_json: json!([]),
            characters_json: json!(["甲"]),
            objects_json: json!(["伞"]),
            key_events_json: json!(["交换信件"]),
            entry_state_json: json!({"门": "关闭"}),
            exit_state_json: json!({"门": "打开"}),
            created_at: NaiveDateTime::MIN,
            updated_at: NaiveDateTime::MIN,
        };

        let state = occurrence.derived_state_json();
        assert_eq!(state["chapter_number"], 5);
        assert_eq!(state["occurrence_order"], 6);
        assert_eq!(state["summary"], "雨夜会面");
    }
}
