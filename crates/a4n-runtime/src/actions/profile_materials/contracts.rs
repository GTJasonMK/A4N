use crate::actions::image_generation::GeneratedImageSummary;
use a4n_orchestrator::profile_material::{
    IMAGE_TYPE_OBJECT_MATERIAL, IMAGE_TYPE_SCENE_MATERIAL, PROFILE_KIND_OBJECT, PROFILE_KIND_SCENE,
};
use serde::{Deserialize, Serialize};

pub(super) const KIND_SCENE: &str = PROFILE_KIND_SCENE;
pub(super) const KIND_OBJECT: &str = PROFILE_KIND_OBJECT;
pub(super) const IMAGE_TYPE_SCENE: &str = IMAGE_TYPE_SCENE_MATERIAL;
pub(super) const IMAGE_TYPE_OBJECT: &str = IMAGE_TYPE_OBJECT_MATERIAL;
pub(super) const MANUAL_UPLOAD_MODEL: &str = "manual_upload";
pub(super) const MAX_UPLOAD_IMAGE_BYTES: usize = 20 * 1024 * 1024;

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ListProfileMaterialsInput {
    pub project_id: i64,
    pub kind: Option<String>,
    pub query: Option<String>,
    pub filter: Option<String>,
    pub sort: Option<String>,
    pub image_limit_per_profile: Option<usize>,
}
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SetProfilePrimaryMaterialInput {
    pub project_id: i64,
    pub profile_kind: String,
    pub profile_id: i64,
    pub image_id: Option<i64>,
}
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UploadProfileMaterialImageInput {
    pub project_id: i64,
    pub profile_kind: String,
    pub profile_id: i64,
    pub file_name: String,
    pub mime_type: String,
    pub data_base64: String,
    pub set_primary: bool,
    pub prompt_snapshot: Option<String>,
    pub style_template_id: Option<String>,
    pub style_template_name: Option<String>,
    pub style_template_version: Option<String>,
    pub custom_style: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProfileMaterialGroup {
    pub kind: String,
    pub profile: ProfileMaterialProfileSummary,
    pub material_state: String,
    pub current_profile_content_sha256: String,
    pub image_count: usize,
    pub primary_image_id: Option<i64>,
    pub primary_image: Option<ProfileMaterialImageSummary>,
    pub images: Vec<ProfileMaterialImageSummary>,
    #[serde(skip_serializing)]
    pub(super) newest_image_sort_key: Option<(chrono::NaiveDateTime, i64)>,
    #[serde(skip_serializing)]
    pub(super) has_favorite_image: bool,
}
#[derive(Debug, Clone, Serialize)]
pub struct ProfileMaterialProfileSummary {
    pub id: i64,
    pub project_id: i64,
    pub name: String,
    pub category: String,
    pub importance: String,
    pub first_seen_chapter: i32,
    pub last_seen_chapter: i32,
    pub primary_text: String,
    pub secondary_text: String,
    pub updated_at: Option<String>,
}
#[derive(Debug, Clone, Serialize)]
pub struct ProfileMaterialImageSummary {
    pub image: GeneratedImageSummary,
    pub is_primary: bool,
    pub staleness: String,
    pub style_template_id: Option<String>,
    pub style_template_name: Option<String>,
}

#[derive(Debug, Clone)]
pub(super) struct ProfileMaterialSource {
    pub(super) kind: &'static str,
    pub(super) profile: ProfileMaterialProfileSummary,
    pub(super) content_hash: String,
}
#[derive(Debug)]
pub(super) struct UploadedProfileMaterialImage {
    pub(super) bytes: Vec<u8>,
    pub(super) mime_type: &'static str,
    pub(super) extension: &'static str,
    pub(super) width: i32,
    pub(super) height: i32,
    pub(super) source_file_name: String,
}
