//! 章节视频提示词实体

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

pub const PROMPT_TYPE_FRAME_SEGMENT_PLAN: &str = "frame_segment_plan";
pub const PROMPT_TYPE_FRAME: &str = "frame_prompt";
pub const PROMPT_TYPE_FIRST_FRAME: &str = "first_frame_prompt";
pub const PROMPT_TYPE_CHAT_VIDEO_FRAME: &str = "chat_video_frame_prompt";
pub const PROMPT_TYPE_STORYBOARD: &str = "storyboard_prompt";
pub const PROMPT_TYPE_VIDEO: &str = "video_prompt";

pub const WORKFLOW_MODE_SHARED_SEGMENT_PLAN: &str = "shared_segment_plan";
pub const WORKFLOW_MODE_BOUNDARY_FRAMES: &str = "boundary_frames";
pub const WORKFLOW_MODE_FIRST_FRAME_CHAIN: &str = "first_frame_chain";
pub const WORKFLOW_MODE_CHAT_VIDEO: &str = "chat_video";
pub const WORKFLOW_MODE_STORYBOARD: &str = "storyboard";
pub const WORKFLOW_MODES: [&str; 5] = [
    WORKFLOW_MODE_SHARED_SEGMENT_PLAN,
    WORKFLOW_MODE_BOUNDARY_FRAMES,
    WORKFLOW_MODE_FIRST_FRAME_CHAIN,
    WORKFLOW_MODE_CHAT_VIDEO,
    WORKFLOW_MODE_STORYBOARD,
];

pub const STATUS_PENDING: &str = "pending";
pub const STATUS_RUNNING: &str = "running";
pub const STATUS_FAILED: &str = "failed";
pub const STATUS_INTERRUPTED: &str = "interrupted";
pub const STATUS_COMPLETED: &str = "completed";
pub const STATUS_CANCELLED: &str = "cancelled";

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "chapter_video_prompt")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub project_id: i64,
    pub chapter_id: i64,

    pub style: Option<String>,

    /// frame_segment_plan / frame_prompt / first_frame_prompt /
    /// chat_video_frame_prompt / storyboard_prompt / video_prompt
    pub prompt_type: String,

    /// shared_segment_plan / boundary_frames / first_frame_chain / chat_video / storyboard
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

    /// pending / running / interrupted / completed / failed / cancelled
    pub status: String,

    pub generated_video_id: Option<i64>,

    /// 已提交插入片段对应的稳定时间线项。
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
