//! 章节视频提示词实体

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "chapter_video_prompt")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub project_id: i64,
    pub chapter_id: i64,

    pub style: Option<String>,

    /// analysis / shot_plan / video_prompt
    pub prompt_type: String,

    /// boundary_frames / first_frame_chain
    pub workflow_mode: String,

    pub shot_number: i32,
    pub title: Option<String>,

    #[sea_orm(column_type = "Text")]
    pub scene_summary: Option<String>,

    #[sea_orm(column_type = "Text")]
    pub visual_prompt: String,

    #[sea_orm(column_type = "Text")]
    pub motion_prompt: Option<String>,

    #[sea_orm(column_type = "Text")]
    pub negative_prompt: Option<String>,

    pub duration_secs: i32,
    pub aspect_ratio: Option<String>,

    #[sea_orm(column_type = "Json")]
    pub reference_image_ids_json: Option<serde_json::Value>,

    /// pending / running / completed / failed / cancelled
    pub status: String,

    pub generated_video_id: Option<i64>,

    /// Canonical timeline item for a committed inserted-segment prompt.
    pub timeline_item_id: Option<i64>,

    #[sea_orm(column_type = "Json")]
    pub metadata_json: Option<serde_json::Value>,

    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::chapter::Entity",
        from = "Column::ChapterId",
        to = "super::chapter::Column::Id"
    )]
    Chapter,
}

impl ActiveModelBehavior for ActiveModel {}

impl Related<super::chapter::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Chapter.def()
    }
}
