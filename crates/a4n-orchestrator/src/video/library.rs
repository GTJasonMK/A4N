use crate::profile_material::{
    PROFILE_KIND_OBJECT, PROFILE_KIND_SCENE, generated_image_profile_material_identity,
};
use a4n_core::models::ai::generated_image;
use std::collections::HashSet;

use super::{
    ChapterVideoSourceContext, OrchestratorContext, is_project_reusable_video_material_image,
};

mod candidates;
mod context;
mod projection;
mod relevance;
mod validation;

use candidates::*;
use projection::video_library_image_entry;
use relevance::*;
use validation::ensure_video_asset_image_usable;
pub(super) use validation::{available_video_image_ids, ensure_video_reference_ids_known};

pub(super) const MAX_VIDEO_LIBRARY_IMAGE_IDS: usize = 32;
pub(super) const MAX_REFERENCE_IMAGE_IDS: usize = 16;
const MAX_AUTO_PROFILE_ASSETS: usize = 24;
const MAX_AVAILABLE_VIDEO_ASSETS: usize = 48;
const SELECTION_REASON_REFERENCE: &str = "reference";
const SELECTION_REASON_MANUAL_LIBRARY: &str = "manual_library";
const SELECTION_REASON_CHAPTER_MATERIAL: &str = "chapter_material";
const SELECTION_REASON_PROFILE_PRIMARY: &str = "profile_primary";
const SELECTION_REASON_PROFILE_FAVORITE: &str = "profile_favorite";
const SELECTION_REASON_PROFILE_LATEST: &str = "profile_latest";

pub fn resolve_video_reference_image_project_ids(
    requested_project_ids: Vec<i64>,
    allowed_project_ids: &[i64],
) -> Result<Vec<i64>, String> {
    let allowed = allowed_project_ids.iter().copied().collect::<HashSet<_>>();
    let mut project_ids = if requested_project_ids.is_empty() {
        allowed_project_ids.to_vec()
    } else {
        let mut resolved = Vec::with_capacity(requested_project_ids.len());
        for (index, project_id) in requested_project_ids.into_iter().enumerate() {
            if project_id <= 0 {
                return Err(format!("image_project_ids[{index}] 必须为正整数"));
            }
            if !allowed.contains(&project_id) {
                return Err(format!(
                    "image_project_ids[{index}] 不属于当前视频项目允许的素材项目：{project_id}"
                ));
            }
            resolved.push(project_id);
        }
        resolved
    };
    project_ids.sort_unstable();
    project_ids.dedup();
    Ok(project_ids)
}

pub fn video_reference_scope_rank(image: &generated_image::Model, chapter_id: i64) -> i32 {
    if image.chapter_id == Some(chapter_id) {
        return 0;
    }
    if generated_image_profile_material_identity(image).is_some() || image.chapter_id.is_none() {
        return 1;
    }
    2
}

pub fn is_video_reference_image_candidate(image: &generated_image::Model, chapter_id: i64) -> bool {
    image.chapter_id == Some(chapter_id)
        || image.chapter_id.is_none()
        || is_project_reusable_video_material_image(&image.image_type, image.metadata_json.as_ref())
}

pub fn image_type_rank(image_type: &str) -> i32 {
    match image_type.trim() {
        "video_start_frame" | "video_end_frame" | "video_tail_frame" => 0,
        "manga_page" | "manga_panel" | "comic_panel" => 1,
        "scene_material" => 2,
        "object_material" => 3,
        "portrait" | "character_portrait" => 4,
        _ => 99,
    }
}

#[cfg(test)]
mod tests;
