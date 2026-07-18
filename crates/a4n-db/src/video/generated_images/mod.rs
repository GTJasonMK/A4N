use a4n_core::models::{
    ai::generated_image,
    asset::{object_profile, profile_primary_material},
    character::protagonist_profile,
    project::{blueprint_character, novel_blueprint},
    scene::scene_profile,
};
use chrono::NaiveDateTime;
use serde_json::Value;

use super::{
    VideoDbError, lock_generated_media_project, normalize_optional_video_record_text,
    required_video_record_text, validate_positive_video_i64, validate_positive_video_ids,
};

mod create;
mod records;
mod references;

pub use create::*;
pub use records::*;
pub use references::*;

pub(super) const IMAGE_TYPE_SCENE_MATERIAL: &str = "scene_material";
pub(super) const IMAGE_TYPE_OBJECT_MATERIAL: &str = "object_material";

pub struct CreateGeneratedImageRecordInput {
    pub project_id: i64,
    pub image_type: String,
    pub file_path: String,
    pub prompt: String,
    pub model: String,
    pub width: i32,
    pub height: i32,
    pub file_size: Option<i64>,
    pub chapter_id: Option<i64>,
    pub panel_id: Option<String>,
    pub metadata_json: Option<Value>,
    pub created_at: NaiveDateTime,
}

#[derive(Debug, Clone)]
pub struct CreateGeneratedImageRecordDraftInput {
    pub project_id: i64,
    pub image_type: String,
    pub file_path: String,
    pub prompt: String,
    pub model: String,
    pub width: i32,
    pub height: i32,
    pub file_size: Option<i64>,
    pub chapter_id: Option<i64>,
    pub panel_id: Option<String>,
    pub metadata_json: Option<Value>,
    pub created_at: Option<NaiveDateTime>,
}

#[derive(Debug, Clone, Default)]
pub struct UpdateGeneratedImageRecordInput {
    pub prompt: Option<String>,
    pub image_type: Option<String>,
    pub chapter_id: Option<i64>,
    pub clear_chapter_id: bool,
    pub panel_id: Option<String>,
    pub clear_panel_id: bool,
    pub ai_score: Option<f32>,
    pub clear_ai_score: bool,
    pub is_favorite: Option<bool>,
}

#[derive(Debug, Clone, Default)]
pub struct VideoProfileMaterialReadModel {
    pub scenes: Vec<scene_profile::Model>,
    pub objects: Vec<object_profile::Model>,
    pub primary_materials: Vec<profile_primary_material::Model>,
    pub material_images: Vec<generated_image::Model>,
}

#[derive(Debug, Clone, Default)]
pub struct VideoProjectContextReadModel {
    pub blueprint: Option<novel_blueprint::Model>,
    pub blueprint_characters: Vec<blueprint_character::Model>,
    pub protagonist_profiles: Vec<protagonist_profile::Model>,
}
