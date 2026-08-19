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

pub use create::{
    create_generated_image_record, create_generated_image_record_with_generation_run_checkpoint,
};
pub(crate) use records::clear_generated_image_references_record;
pub(crate) use records::generated_images_for_chapter_by_id_desc;
pub use records::{
    delete_generated_image_record_with_cleanup, delete_manga_page_image_records,
    generated_image_by_id, generated_images_by_ids, generated_images_for_chapter,
    generated_images_for_project, generated_images_for_projects_chapter_by_id_desc,
    generated_images_for_video_prompt_references, update_generated_image_record,
    video_library_chapter_image_records, video_library_explicit_image_records,
    video_profile_material_read_model, video_project_context_read_model,
};
pub(in crate::video) use references::video_project_record;
pub use references::{
    ensure_image_chapter_belongs_to_project_record, ensure_video_chapter_belongs_to_project_record,
    ensure_video_reference_images_available_record, validated_video_reference_image_records,
    video_asset_project_ids_record, video_chapter_record, video_reference_candidate_image_records,
    video_tail_frame_images,
};
use references::{
    normalize_optional_image_record_text, required_image_record_text,
    validate_generated_image_ai_score, validate_positive_image_i32, validate_positive_image_i64,
};

pub(in crate::video::generated_images) struct CreateGeneratedImageRecordInput {
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
