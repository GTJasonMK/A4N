use a4n_db_models::models::{ai::generated_image, project::novel_blueprint};
use a4n_domain::media::CreateGeneratedImageRecordDraftInput;
use chrono::NaiveDateTime;
use sea_orm::FromQueryResult;
use serde_json::Value;

use super::{
    VideoDbError, lock_generated_media_project, normalize_optional_video_record_text,
    required_video_record_text, validate_positive_video_ids,
};

mod create;
mod publication;
mod records;
mod references;

pub use create::create_generated_image_record;
pub(crate) use publication::reclaim_image_publications_for_deleted_images;
pub use publication::{
    ImagePublicationRequest, active_image_publications_for_paths, create_image_publication_intent,
    image_publication_by_id, image_publication_by_image_id, image_publications_for_recovery,
    image_publications_for_recovery_for_model, mark_image_publication_reclaimed,
    mark_image_publication_reclaiming, project_has_active_image_publication,
    publish_image_publication, publish_image_publication_with_generation_run_checkpoint,
    update_image_publication_metadata, validate_image_publication_request,
};
pub(crate) use records::clear_generated_image_references_record;
pub(crate) use records::generated_images_for_chapter_by_id_desc;
pub use records::{
    delete_generated_image_record_with_cleanup, delete_manga_page_image_records,
    generated_image_by_id, generated_image_ids_by_ids, generated_images_by_ids,
    generated_images_for_chapter, generated_images_for_project, generated_images_for_projects,
    generated_images_for_projects_chapter_by_id_desc, update_generated_image_record,
    video_library_chapter_image_records, video_library_explicit_image_records,
    video_profile_material_read_model, video_project_context_read_model,
};
pub(crate) use references::validated_video_reference_image_records_batch;
pub(crate) use references::video_project_record;
pub use references::{
    VideoProjectIdentity, ensure_image_chapter_belongs_to_project_record,
    ensure_video_chapter_belongs_to_project_record, ensure_video_reference_images_available_record,
    validated_video_reference_image_records, video_asset_project_ids_record, video_chapter_record,
    video_reference_candidate_image_records, video_tail_frame_images,
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
pub struct VideoProfileRelevanceRow {
    pub id: i64,
    pub name: String,
    pub aliases_json: Value,
    pub first_seen_chapter: i32,
    pub last_seen_chapter: i32,
}

#[derive(Debug, Clone)]
pub struct VideoProfilePrimaryMaterialRow {
    pub profile_kind: String,
    pub profile_id: i64,
    pub primary_image_id: i64,
}

#[derive(Debug, Clone, Default)]
pub struct VideoProfileMaterialReadModel {
    pub scenes: Vec<VideoProfileRelevanceRow>,
    pub objects: Vec<VideoProfileRelevanceRow>,
    pub primary_materials: Vec<VideoProfilePrimaryMaterialRow>,
    pub material_images: Vec<generated_image::Model>,
}

#[derive(Debug, Clone, Default)]
pub struct VideoProjectContextReadModel {
    pub blueprint: Option<novel_blueprint::Model>,
    pub blueprint_characters: Vec<VideoPromptBlueprintCharacterRecord>,
    pub protagonist_profiles: Vec<VideoPromptProtagonistProfileRecord>,
}

#[derive(Debug, Clone, FromQueryResult)]
pub struct VideoPromptBlueprintCharacterRecord {
    pub name: String,
    pub character_type: String,
    pub gender: Option<String>,
    pub age: Option<String>,
    pub appearance: Option<String>,
    pub personality: Option<String>,
    pub background: Option<String>,
    pub motivation: Option<String>,
    pub relationship_to_protagonist: Option<String>,
}

#[derive(Debug, Clone, FromQueryResult)]
pub struct VideoPromptProtagonistProfileRecord {
    pub name: String,
    pub gender: Option<String>,
    pub age: Option<String>,
    pub base_attributes: Value,
    pub current_state: Value,
    pub appearance: Option<String>,
    pub background: Option<String>,
    pub personality: Option<String>,
}
