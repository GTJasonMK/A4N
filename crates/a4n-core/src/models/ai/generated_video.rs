//! 生成视频实体

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "generated_video")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub project_id: i64,
    pub chapter_id: Option<i64>,

    /// 视频类型：chapter_video / shot_video / trimmed_clip
    pub video_type: String,

    /// 视频工作流模式：boundary_frames / first_frame_chain / reference_media / chat_video
    pub workflow_mode: Option<String>,

    /// 文件路径
    pub file_path: Option<String>,

    /// 缩略图路径
    pub thumbnail_path: Option<String>,

    /// 生成提示词
    #[sea_orm(column_type = "Text")]
    pub prompt: String,

    #[sea_orm(column_type = "Text")]
    pub negative_prompt: Option<String>,

    /// 使用的模型
    pub model: String,

    pub width: i32,
    pub height: i32,
    pub duration_secs: f32,
    pub fps: i32,

    /// 文件大小（字节）
    pub file_size: Option<i64>,

    pub source_video_id: Option<i64>,
    pub source_image_id: Option<i64>,

    /// pending / running / interrupted / completed / failed / cancelled
    pub status: String,
    pub provider_job_id: Option<String>,

    #[sea_orm(column_type = "Text")]
    pub error_message: Option<String>,

    #[sea_orm(column_type = "Json")]
    pub metadata_json: Option<serde_json::Value>,

    /// 是否被用户收藏
    pub is_favorite: bool,

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
    #[sea_orm(
        belongs_to = "super::super::chapter::chapter::Entity",
        from = "Column::ChapterId",
        to = "super::super::chapter::chapter::Column::Id"
    )]
    Chapter,
}

impl ActiveModelBehavior for ActiveModel {}

impl Related<super::super::project::novel_project::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::NovelProject.def()
    }
}

impl Related<super::super::chapter::chapter::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Chapter.def()
    }
}
