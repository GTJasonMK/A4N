use crate::profile_material::{
    PROFILE_KIND_OBJECT, PROFILE_KIND_SCENE, generated_image_profile_material_identity,
};

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

#[cfg(test)]
mod tests;
