use std::collections::{BTreeSet, HashMap};

use a4n_api_contract::image::GeneratedImageSummary;
use a4n_api_contract::manga::MangaReferenceBindingSummary;
use a4n_canvas_contract::GenerateCanvasImageInput;
use a4n_config::{ensure_image_size_supported, image_sizes_from_json};
use a4n_db_models::models::{
    asset::object_profile,
    chapter::chapter_manga_prompt::{self, MANGA_PAGE_PROMPT_TYPE, Model as MangaPagePrompt},
    project::novel_project,
    scene::scene_profile,
};
use a4n_domain::source_text_hash as source_content_sha256;
use a4n_domain::{
    image::{
        IMAGE_TYPE_CANVAS, IMAGE_TYPE_OBJECT_MATERIAL, IMAGE_TYPE_SCENE_MATERIAL,
        batch_item_reference_image_ids, video_frame_image_generation_context,
    },
    profile_material::ProfileMaterialKind,
    source_spans::{ChapterSourceContext, source_metadata},
};
use a4n_error::AppError;
use a4n_image_contract::{ImageGenerationRequest, ImageReference};
use serde_json::{Value, json};

use crate::{
    OrchestratorContext,
    manga::{ensure_manga_source_can_write, manga_page_generation_layout},
    profile_material::{
        ProfileMaterialStyleMetadata, object_profile_content, object_profile_image_prompt,
        profile_material_metadata, scene_profile_content, scene_profile_image_prompt,
    },
    project::ChapterMangaSourceSummary,
};
use a4n_domain::collaboration::ProjectAccessRole;

use super::{
    super::{
        super::{BatchImageExecutionItem, BatchImageGenerationApplicationInput},
        batch::rules::{
            attach_video_frame_reference_image_ids, ensure_batch_image_item_shape,
            ensure_batch_reference_image_backend, image_size_for_batch_item,
            optional_json_i64_field, optional_json_string_field,
        },
    },
    contracts::{
        GenerateBatchImagesInput, GenerateGalgameImageInput, GeneratePageImageInput,
        GenerateSceneImageInput, ImageGenerationExecutionInput, ImageGenerationHostPort,
        ImageGenerationPostCommitAction, ImageGenerationProgress, ImageGenerationWorkflowBinding,
        ObjectProfileImageInput, ObjectProfileImagePromptOutput, PreparedImageGenerationRequest,
        SceneProfileImageInput, SceneProfileImagePromptOutput,
    },
};

const IMAGE_METADATA_SCHEMA_VERSION: u64 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ImageChapterBindingAccess {
    ReadOnly,
    Writable,
}

struct DirectImagePlan {
    project_id: i64,
    prompt: String,
    image_type: String,
    config_id: Option<i64>,
    width: i32,
    height: i32,
    chapter_id: Option<i64>,
    panel_id: Option<String>,
    metadata_json: Option<Value>,
    workflow_run_id: Option<i64>,
    command: String,
    file_extra: Option<String>,
    file_context: String,
}

struct PreparedDirectImage {
    plan: DirectImagePlan,
    preview: PreparedImageGenerationRequest,
    chapter_id: Option<i64>,
    metadata_json: Option<Value>,
}

struct PreparedPageImage {
    preview: PreparedImageGenerationRequest,
    chapter: ChapterMangaSourceSummary,
    page_prompt: MangaPagePrompt,
    panel_id: String,
    metadata_json: Value,
}

#[derive(Clone, Copy)]
struct ProfileImageSpec {
    profile_kind: ProfileMaterialKind,
    image_type: &'static str,
    command: &'static str,
}

const OBJECT_PROFILE_IMAGE_SPEC: ProfileImageSpec = ProfileImageSpec {
    profile_kind: ProfileMaterialKind::Object,
    image_type: IMAGE_TYPE_OBJECT_MATERIAL,
    command: "image_object_profile",
};
const SCENE_PROFILE_IMAGE_SPEC: ProfileImageSpec = ProfileImageSpec {
    profile_kind: ProfileMaterialKind::Scene,
    image_type: IMAGE_TYPE_SCENE_MATERIAL,
    command: "image_scene_profile",
};

struct ProfileImageMaterial {
    spec: ProfileImageSpec,
    profile_id: i64,
    project_id: i64,
    name: String,
    prompt: String,
    profile_content: String,
    source_material: String,
    style_metadata: ProfileMaterialStyleMetadata,
}

struct PreparedBatchImageJob {
    execution: BatchImageExecutionItem,
    preview_notes: Vec<String>,
}

include!("generation.rs");
include!("plans.rs");
include!("metadata.rs");
